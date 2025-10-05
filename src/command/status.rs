use jiff::Timestamp;
use serde::Serialize;

use crate::io::gpio::get_battery_value;
use crate::io::gpio::get_sensor_value;
use crate::net::mqtt::latency;
use crate::power::charge_level;
use crate::power::humidity_level;
use crate::time::get_last_watered;
use crate::time::now;
use crate::watering::get_low_humidity_limit;

#[derive(Serialize)]
pub struct Status {
    pub latency_ms: u64,
    pub humidity: u32,
    pub humidity_raw: u16,
    pub charge: u32,
    pub charge_raw: u16,
    pub low_humidity_limit: u16,
    pub last_watered_timestamp: Timestamp,
    pub report_timestamp: Timestamp,
}

pub async fn get_status() -> Status {
    Status {
        latency_ms: latency().await.unwrap_or(0),
        humidity: humidity_level().await,
        humidity_raw: get_sensor_value().await,
        charge: charge_level().await,
        charge_raw: get_battery_value().await,
        low_humidity_limit: get_low_humidity_limit().await,
        last_watered_timestamp: get_last_watered().await,
        report_timestamp: now().await.unwrap_or(Timestamp::constant(0, 0)),
    }
}
