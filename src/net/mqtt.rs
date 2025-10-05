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
use esp_println::println;
use heapless::String;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::publish_packet::QualityOfService;

use crate::command::Command;
use crate::command::status::get_status;
use crate::display::STATUS_LEN;
use crate::display::update_status;
use crate::error::{NetError, SysError};

static LATENCY: Mutex<CriticalSectionRawMutex, Duration> = Mutex::new(Duration::from_secs(0));

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

    let mut status: String<STATUS_LEN> = String::new();

    let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(MQTT_REFRESH_TIME));

    let address = match stack.dns_query(MQTT_SERVER, DnsQueryType::A).await {
        Ok(addr) => addr,
        Err(e) => {
            write!(status, "MQTT DNS: {:?}", e).ok();
            update_status(&status).await.ok();
            return Err(SysError::Net(NetError::Resolve));
        }
    };

    let remote_endpoint: IpEndpoint = (address[0], MQTT_PORT).into();
    if let Err(e) = socket.connect(remote_endpoint).await {
        write!(status, "MQTT Sock: {:?}", e).ok();
        update_status(&status).await.ok();
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
        write!(status, "MQTT Conn: {:?}", e).ok();
        update_status(&status).await.ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    if let Err(e) = client.send_ping().await {
        write!(status, "MQTT Ping: {:?}", e).ok();
        update_status(&status).await.ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    let msg: String<MQTT_BUFFER_SIZE> = serde_json_core::to_string(&get_status().await).unwrap();
    println!("Status: {}", msg);

    if let Err(e) = client
        .send_message(MQTT_TOPIC, msg.as_bytes(), QualityOfService::QoS1, true)
        .await
    {
        write!(status, "MQTT Pub: {:?}", e).ok();
        update_status(&status).await.ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    if let Err(e) = client.subscribe_to_topic("water/control").await {
        write!(status, "MQTT Sub: {:?}", e).ok();
        update_status(&status).await.ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    write!(status, "MQTT wait...").ok();
    update_status(&status).await.ok();
    match client.receive_message().await {
        Ok((_topic, payload)) => {
            let command: Result<(Command, _), _> = serde_json_core::from_slice(payload);
            if let Ok((cmd, _)) = command {
                cmd.process().await;
            } else {
                update_status("MQTT: cmd err")
                    .await
                    .map_err(|_| NetError::Mqtt)?;
            }
            Ok(())
        }
        Err(e) => {
            write!(status, "MQTT Recv: {:?}", e).ok();
            update_status(&status).await.ok();
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
