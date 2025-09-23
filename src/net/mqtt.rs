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

const MQTT_REFRESH_TIME: Duration = Duration::from_secs(120);
const MQTT_ERR_REFRESH_TIME: Duration = Duration::from_secs(20);
const MQTT_SERVER: &str = "raspberrypi.jp.home.rayslava.com";
const MQTT_USER: &str = env!("MQTT_USER");
const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");
const MQTT_PORT: u16 = 1883;
const MQTT_CLIENT_ID: &str = "water_machine";
const MQTT_TOPIC: &str = "water/status";

async fn update_mqtt(
    config: &ClientConfig<'_, 10, Rng>,
    stack: &'static Stack<'static>,
) -> Result<(), SysError> {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let count = 42usize;

    let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(MQTT_REFRESH_TIME));

    let address = stack.dns_query(MQTT_SERVER, DnsQueryType::A).await?;
    let remote_endpoint: IpEndpoint = (address[0], MQTT_PORT).into();
    socket.connect(remote_endpoint).await?;

    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client = MqttClient::new(
        socket,
        &mut write_buffer,
        80,
        &mut recv_buffer,
        80,
        config.clone(),
    );

    client
        .connect_to_broker()
        .await
        .map_err(|_| NetError::Mqtt)?;

    let mut msg: String<32> = String::new();
    write!(msg, "{:.2}", count).map_err(|_| NetError::Mqtt)?;
    client
        .send_message(MQTT_TOPIC, msg.as_bytes(), QualityOfService::QoS1, true)
        .await
        .map_err(|_| NetError::Mqtt)?;

    client
        .subscribe_to_topic("water/control")
        .await
        .map_err(|_| NetError::Mqtt)?;

    match client.receive_message().await {
        Ok((_topic, payload)) => {
            let mut response: String<STATUS_LEN> = String::new();
            let width = STATUS_LEN.min(payload.len());
            write!(
                response,
                "MQTT: {:<width$}",
                core::str::from_utf8(&payload[..width]).unwrap(),
            )
            .map_err(|_| NetError::Mqtt)?;
            update_status(&response).await.map_err(|_| NetError::Mqtt)?;
            Ok(())
        }
        _ => Err(SysError::Net(NetError::Mqtt)),
    }
}

#[embassy_executor::task]
pub async fn mqtt_task(rng: Rng, stack: &'static Stack<'static>) {
    let mut config = ClientConfig::new(rust_mqtt::client::client_config::MqttVersion::MQTTv5, rng);
    config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_client_id(MQTT_CLIENT_ID);
    config.max_packet_size = 100;
    config.add_username(MQTT_USER);
    config.add_password(MQTT_PASSWORD);

    loop {
        if let Err(e) = update_mqtt(&config, stack).await {
            let mut status: String<STATUS_LEN> = String::new();
            write!(status, "MQTT err: {:?}", e).ok();
            update_status(&status).await.ok();
            Timer::after(MQTT_ERR_REFRESH_TIME).await;
        };

        *LATENCY.lock().await = measure_latency(stack)
            .await
            .unwrap_or(Duration::from_secs(0));
    }
}
