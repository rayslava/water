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
use heapless::String;
use rust_mqtt::Bytes;
use rust_mqtt::buffer::AllocBuffer;
use rust_mqtt::client::Client;
use rust_mqtt::client::event::Event;
use rust_mqtt::client::options::{
    ConnectOptions, PublicationOptions, RetainHandling, SubscriptionOptions,
};
use rust_mqtt::config::{KeepAlive, SessionExpiryInterval};
use rust_mqtt::types::{MqttBinary, MqttString, QoS, TopicFilter, TopicName};

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
const MQTT_CONTROL_TOPIC: &str = "water/control";
const MQTT_BUFFER_SIZE: usize = 1024;

async fn update_mqtt(stack: &'static Stack<'static>) -> Result<(), SysError> {
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

    let mut alloc = AllocBuffer;
    let mut client: Client<'_, _, AllocBuffer, 1, 1, 1> = Client::new(&mut alloc);

    // Safety: string literals are valid UTF-8 and within MQTT length limits
    let client_id = unsafe { MqttString::from_slice_unchecked(MQTT_CLIENT_ID) };
    let username = unsafe { MqttString::from_slice_unchecked(MQTT_USER) };
    let password =
        unsafe { MqttBinary::from_slice_unchecked(MQTT_PASSWORD.as_bytes()) };

    let connect_options = ConnectOptions {
        clean_start: true,
        keep_alive: KeepAlive::Seconds(60),
        session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
        user_name: Some(username),
        password: Some(password),
        will: None,
    };

    if let Err(e) = client.connect(socket, &connect_options, Some(client_id)).await {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "{:?}", e).ok();
        return Err(SysError::Net(NetError::Mqtt));
    }

    // Publish status message with QoS 1
    let msg =
        serde_json_core::to_string::<_, MQTT_BUFFER_SIZE>(&get_status().await).unwrap();

    // Safety: MQTT_TOPIC is a valid topic name
    let topic =
        unsafe { TopicName::new_unchecked(MqttString::from_slice_unchecked(MQTT_TOPIC)) };
    let pub_options = PublicationOptions {
        retain: true,
        topic,
        qos: QoS::AtLeastOnce,
    };

    let pub_pid = match client
        .publish(&pub_options, Bytes::from(msg.as_bytes()))
        .await
    {
        Ok(pid) => pid,
        Err(e) => {
            let mut status = STATUS.lock().await;
            status.clear();
            write!(status, "{:?}", e).ok();
            return Err(SysError::Net(NetError::Mqtt));
        }
    };

    // Wait for QoS 1 publish acknowledgement
    loop {
        match client.poll().await {
            Ok(Event::PublishAcknowledged(ack)) if ack.packet_identifier == pub_pid => break,
            Ok(Event::PublishRejected(_)) => return Err(SysError::Net(NetError::Mqtt)),
            Ok(_) => {}
            Err(e) => {
                let mut status = STATUS.lock().await;
                status.clear();
                write!(status, "{:?}", e).ok();
                return Err(SysError::Net(NetError::Mqtt));
            }
        }
    }

    {
        let mut status = STATUS.lock().await;
        status.clear();
        write!(status, "OK").ok();
    }

    // Subscribe to control topic
    // Safety: MQTT_CONTROL_TOPIC is a valid topic filter
    let filter = unsafe {
        TopicFilter::new_unchecked(MqttString::from_slice_unchecked(MQTT_CONTROL_TOPIC))
    };
    let sub_options = SubscriptionOptions {
        retain_handling: RetainHandling::AlwaysSend,
        retain_as_published: false,
        no_local: false,
        qos: QoS::AtLeastOnce,
    };

    let sub_pid = match client.subscribe(filter, sub_options).await {
        Ok(pid) => pid,
        Err(e) => {
            let mut status = STATUS.lock().await;
            status.clear();
            write!(status, "{:?}", e).ok();
            return Err(SysError::Net(NetError::Mqtt));
        }
    };

    // Wait for subscription acknowledgement
    loop {
        match client.poll().await {
            Ok(Event::Suback(ack)) if ack.packet_identifier == sub_pid => break,
            Ok(_) => {}
            Err(e) => {
                let mut status = STATUS.lock().await;
                status.clear();
                write!(status, "{:?}", e).ok();
                return Err(SysError::Net(NetError::Mqtt));
            }
        }
    }

    // Wait for an incoming command
    loop {
        match client.poll().await {
            Ok(Event::Publish(publication)) => {
                let command: Result<(Command, _), _> =
                    serde_json_core::from_slice(&publication.message);
                if let Ok((cmd, _)) = command {
                    let mut status = STATUS.lock().await;
                    status.clear();
                    write!(status, "Cmd").ok();
                    drop(status);
                    cmd.process().await;
                } else {
                    let mut status = STATUS.lock().await;
                    status.clear();
                    write!(status, "ERecv").ok();
                    return Err(SysError::Net(NetError::Mqtt));
                }
                return Ok(());
            }
            Ok(_) => {}
            Err(e) => {
                let mut status = STATUS.lock().await;
                status.clear();
                write!(status, "{:?}", e).ok();
                return Err(SysError::Net(NetError::Mqtt));
            }
        }
    }
}

#[embassy_executor::task]
pub async fn mqtt_task(stack: &'static Stack<'static>) {
    loop {
        *LATENCY.lock().await = measure_latency(stack)
            .await
            .unwrap_or(Duration::from_secs(0));

        if update_mqtt(stack).await.is_err() {
            Timer::after(MQTT_ERR_REFRESH_TIME).await;
        };
    }
}
