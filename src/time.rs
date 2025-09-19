use jiff::{
    Timestamp,
    civil::Time,
    tz::{self, TimeZone},
};

use crate::{error::SysError, io::rtc::get_time};

static TZ: TimeZone = tz::get!("Asia/Tokyo");

pub async fn localtime() -> Result<Time, SysError> {
    let now = Timestamp::from_microsecond(get_time().await? as i64)?;
    Ok(now.to_zoned(TZ.clone()).time())
}
