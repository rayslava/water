use core::fmt::Write;
use embassy_net::dns::DnsQueryType;
use embassy_net::icmp::PacketMetadata;
use embassy_net::icmp::ping::{PingManager, PingParams};
use embassy_net::tcp::TcpSocket;
use embassy_net::{IpEndpoint, Stack};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Duration;
use embassy_time::Timer;
use esp_hal::rng::Rng;
use heapless::String;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::publish_packet::QualityOfService;

use crate::command::Command;
use crate::command::status::get_status;
use crate::error::{ConversionError, NetError, SysError};

const MQTT_STATUS_LEN: usize = 10;
static LATENCY: Mutex<CriticalSectionRawMutex, Duration> = Mutex::new(Duration::from_secs(0));
static STATUS: Mutex<CriticalSectionRawMutex, String<MQTT_STATUS_LEN>> =
    Mutex::new(String::<MQTT_STATUS_LEN>::new());

pub async fn mqtt_status() -> Result<String<MQTT_STATUS_LEN>, ConversionError> {
    Ok(STATUS.lock().await.clone())
}

async fn measure_latency(stack: &Stack<'_>) -> Result<Duration, NetError> {
    let mut rx_buffer = [0; 256];
    let mut tx_buffer = [0; 256];
    let mut rx_meta = [PacketMetadata::EMPTY];
    let mut tx_meta = [PacketMetadata::EMPTY];

    let mut ping_manager = PingManager::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    if let Some(config) = stack.config_v4()
        && let Some(gateway) = config.gateway
    {
        let mut ping_params = PingParams::new(gateway);
        ping_params.set_payload(b"Watering machine");
        match ping_manager.ping(&ping_params).await {
            Ok(time) => Ok(time),
            Err(_) => Err(NetError::Ping),
        }
    } else {
        Err(NetError::Ping)
    }
}

pub async fn latency() -> Result<u64, NetError> {
    let ping = (*LATENCY.lock().await).as_millis();
    if ping > 0 {
        Ok(ping)
    } else {
        Err(NetError::Ping)
    }
}

const MQTT_REFRESH_TIME: Duration = Duration::from_secs(10);
const MQTT_ERR_REFRESH_TIME: Duration = Duration::from_secs(5);
const MQTT_SERVER: &str = "raspberrypi.jp.home.rayslava.com";
const MQTT_USER: &str = env!("MQTT_USER");
const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");
const MQTT_PORT: u16 = 1883;
const MQTT_CLIENT_ID: &str = "water_machine";
const MQTT_TOPIC: &str = "water/status";
const MQTT_BUFFER_SIZE: usize = 1024;

async fn update_mqtt(
    config: ClientConfig<'_, 10, Rng>,
    stack: &'static Stack<'static>,
) -> Result<(), SysError> {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(MQTT_REFRESH_TIME));

    let address = match stack.dns_query(MQTT_SERVER, DnsQueryType::A).await {
        Ok(addr) => addr,
        Err(e) => {
            let mut status = STATUS.lock().await;
            status.clear();
            write!(status, "{:?}", e).ok();
            return Err(SysError::Net(NetError::Resolve));
        }
    };

    let remote_endpoint: IpEndpoint = (address[0], MQTT_PORT).into();
    if let Err(e) = socket.connect(remote_endpoint).await {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "{:?}", e).ok();
        return Err(SysError::Net(NetError::Socket));
    }

    let mut recv_buffer = [0; MQTT_BUFFER_SIZE];
    let mut write_buffer = [0; MQTT_BUFFER_SIZE];

    let mut client = MqttClient::new(
        socket,
        &mut write_buffer,
        MQTT_BUFFER_SIZE,
        &mut recv_buffer,
        MQTT_BUFFER_SIZE,
        config,
    );

    if let Err(e) = client.connect_to_broker().await {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "{:?}", e).ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    if let Err(e) = client.send_ping().await {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "{:?}", e).ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    let msg = serde_json_core::to_string::<_, MQTT_BUFFER_SIZE>(&get_status().await).unwrap();

    if let Err(e) = client
        .send_message(MQTT_TOPIC, msg.as_bytes(), QualityOfService::QoS1, true)
        .await
    {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "{:?}", e).ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "OK").ok();
    }

    if let Err(e) = client.subscribe_to_topic("water/control").await {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "{:?}", e).ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    match client.receive_message().await {
        Ok((_topic, payload)) => {
            let command: Result<(Command, _), _> = serde_json_core::from_slice(payload);
            if let Ok((cmd, _)) = command {
                let mut status = STATUS.lock().await;
                status.clear();
                write!(status, "Cmd").ok();
                cmd.process().await;
            } else {
                let mut status = STATUS.lock().await;
                status.clear();
                write!(status, "ERecv").ok();
                return Err(SysError::Net(NetError::Mqtt));
            }
            Ok(())
        }
        Err(e) => {
            let mut status = STATUS.lock().await;
            status.clear();
            write!(status, "{:?}", e).ok();
            Err(SysError::Net(NetError::Mqtt))
        }
    }
}

#[embassy_executor::task]
pub async fn mqtt_task(rng: Rng, stack: &'static Stack<'static>) {
    let mut config = ClientConfig::new(rust_mqtt::client::client_config::MqttVersion::MQTTv5, rng);
    config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_client_id(MQTT_CLIENT_ID);
    config.max_packet_size = 128;
    config.add_username(MQTT_USER);
    config.add_password(MQTT_PASSWORD);

    loop {
        *LATENCY.lock().await = measure_latency(stack)
            .await
            .unwrap_or(Duration::from_secs(0));

        if update_mqtt(config.clone(), stack).await.is_err() {
            Timer::after(MQTT_ERR_REFRESH_TIME).await;
        };
    }
}
