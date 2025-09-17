use display_interface::DisplayError;
use thiserror_no_std::Error;

#[derive(Debug, Error)]
pub enum I2cError {
    #[error("Can't initialize I2C")]
    InitializationFailed,
}

#[derive(Debug, Error)]
pub enum HwError {
    I2c(#[from] I2cError),
    Display(#[from] DisplayError),
}
