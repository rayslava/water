use crate::error::{HwError, I2cError};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use esp_hal::peripherals::{GPIO21, GPIO22, I2C0};
use esp_hal::{i2c::master::Config, time::Rate};

pub async fn display_i2c_init<'a>(
    i2c: I2C0<'a>,
    sda: GPIO21<'a>,
    scl: GPIO22<'a>,
) -> Result<I2c<'a, Async>, HwError> {
    let config = Config::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(i2c, config)
        .map_err(|_| I2cError::InitializationFailed)?
        .with_scl(scl)
        .with_sda(sda)
        .into_async();
    Ok(i2c)
}
