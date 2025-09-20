use jiff::{
    Timestamp,
    civil::Time,
    tz::{self, TimeZone},
};

use crate::{error::SysError, io::rtc::get_time};

static TZ: TimeZone = tz::get!("Asia/Tokyo");

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
