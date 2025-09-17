use crate::error::HwError;
use embedded_graphics::text::Baseline;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::mode::{BufferedGraphicsModeAsync, DisplayConfigAsync};
use ssd1306::prelude::I2CInterface;
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};

pub type Display<'a> = Ssd1306Async<
    I2CInterface<&'a mut I2c<'a, Async>>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

pub async fn init<'a>(i2c: &'a mut I2c<'a, Async>) -> Result<Display<'a>, HwError> {
    let i2c = I2CDisplayInterface::new(i2c);

    let mut display = Ssd1306Async::new(i2c, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().await?;
    display.clear_buffer();
    display.flush().await?;
    Ok(display)
}

pub async fn update_status(status: &str, display: &mut Display<'_>) -> Result<(), HwError> {
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    display.clear_buffer();

    Text::with_baseline(status, Point::new(10, 0), text_style, Baseline::Top).draw(display)?;

    Ok(display.flush().await?)
}
