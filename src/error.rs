use display_interface::DisplayError;
use esp_wifi::InitializationError;
use esp_wifi::wifi::WifiError;
use thiserror_no_std::Error;

#[derive(Debug, Error)]
pub enum I2cError {
    #[error("Can't initialize I2C")]
    InitializationFailed,
}

#[derive(Debug, Error)]
pub enum GpioError {
    #[error("Can't initialize GPIO")]
    InitializationFailed,
}

#[derive(Debug, Error)]
pub enum HwError {
    I2c(#[from] I2cError),
    Display(#[from] DisplayError),
    WifiInit(#[from] InitializationError),
    Wifi(#[from] WifiError),
    Gpio(#[from] GpioError),
}
