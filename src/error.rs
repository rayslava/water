use core::str::Utf8Error;
use display_interface::DisplayError;
use esp_wifi::InitializationError;
use esp_wifi::wifi::WifiError;
use thiserror_no_std::Error;

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
}

// Generate transitive From implementations
transitive_from!(
    UIError: Utf8Error => Conversion,
    UIError: DisplayError => Hardware,
    UIError: I2cError => Hardware,
    UIError: InitializationError => Hardware,
    UIError: WifiError => Hardware,
    UIError: GpioError => Hardware,
);
