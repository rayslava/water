use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use jiff::{
    Timestamp,
    civil::Time,
    tz::{self, TimeZone},
};

use crate::{error::SysError, io::rtc::get_time};

pub static TZ: TimeZone = tz::get!("Asia/Tokyo");

pub async fn localtime() -> Result<Time, SysError> {
    let timestamp = get_time().await?;
    if timestamp < 1000 * 1000000 {
        // If timestamp is too small we don't have correct local time yet
        Err(SysError::NoTime)
    } else {
        let now = Timestamp::from_microsecond(timestamp as i64)?;
        Ok(now.to_zoned(TZ.clone()).time())
    }
}

pub async fn now() -> Result<Timestamp, SysError> {
    let timestamp = get_time().await?;
    Ok(Timestamp::from_microsecond(timestamp as i64)?)
}

static LAST_WATERED: Mutex<CriticalSectionRawMutex, Timestamp> =
    Mutex::new(Timestamp::constant(0, 0));

pub async fn set_last_watered(time: Timestamp) {
    LAST_WATERED.lock().await.clone_from(&time);
}

pub async fn get_last_watered() -> Timestamp {
    *LAST_WATERED.lock().await
}
