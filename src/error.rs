use core::str::Utf8Error;
use display_interface::DisplayError;
use embassy_executor::SpawnError;
use esp_hal::system::Error as SystemError;
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
pub enum ConversionError {
    Utf(#[from] Utf8Error),
    Format(#[from] core::fmt::Error),
}

#[derive(Debug, Error)]
pub enum HwError {
    I2c(#[from] I2cError),
    Display(#[from] DisplayError),
    WifiInit(#[from] InitializationError),
    Wifi(#[from] WifiError),
    Gpio(#[from] GpioError),
}

#[derive(Debug, Error)]
pub enum UIError {
    Conversion(#[from] ConversionError),
    Hardware(#[from] HwError),
    DrawError,
}

#[derive(Debug, Error)]
pub enum NetError {
    Dns(#[from] embassy_net::dns::Error),
    Resolve,
    Ping,
}

#[derive(Debug, Error)]
pub enum SysError {
    Spawn(#[from] SpawnError),
    Hardware(#[from] HwError),
    System(#[from] SystemError),
    Net(#[from] NetError),
    Time(#[from] jiff::Error),
    TimerSetup,
    NoTime,
}

// Generate transitive From implementations
macro_rules! transitive_from {
    ($($to:ty: $from:ty => $via:ident),* $(,)?) => {
        $(
            impl From<$from> for $to {
                fn from(err: $from) -> Self {
                    Self::$via(err.into())
                }
            }
        )*
    };
}

transitive_from!(
    UIError: Utf8Error => Conversion,
    UIError: core::fmt::Error => Conversion,
    UIError: DisplayError => Hardware,
    UIError: I2cError => Hardware,
    UIError: InitializationError => Hardware,
    UIError: WifiError => Hardware,
    UIError: GpioError => Hardware,
    SysError: InitializationError => Hardware,
    SysError: WifiError => Hardware,
    SysError: embassy_net::dns::Error => Net,
);
