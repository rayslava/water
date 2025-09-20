use embassy_net::Stack;
use embassy_net::icmp::PacketMetadata;
use embassy_net::icmp::ping::{PingManager, PingParams};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};

use crate::error::NetError;

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

const MQTT_REFRESH_TIME: Duration = Duration::from_secs(3);

#[embassy_executor::task]
pub async fn mqtt_task(stack: &'static Stack<'static>) {
    loop {
        *LATENCY.lock().await = measure_latency(stack)
            .await
            .unwrap_or(Duration::from_secs(0));
        Timer::after(MQTT_REFRESH_TIME).await;
    }
}
