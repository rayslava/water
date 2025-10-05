use core::fmt::Write;
use heapless::String;
use serde::{Deserialize, Serialize};

use crate::display::STATUS_LEN;
use crate::display::update_status;
use crate::watering::set_low_humidity_limit;

#[derive(Serialize, Deserialize)]
pub enum Command {
    SetMqttTimeout(u32),
    SetHumidityTrigger(u16),
}

impl Command {
    pub async fn process(&self) {
        let mut status: String<STATUS_LEN> = String::new();

        match self {
            Command::SetMqttTimeout(secs) => {
                write!(status, "MQTT t/o: {}s", secs).ok();
                update_status(&status).await.ok();
            }
            Command::SetHumidityTrigger(hum) => {
                set_low_humidity_limit(*hum).await;
                write!(status, "Hum. lim: {}", hum).ok();
                update_status(&status).await.ok();
            }
        }
    }
}
