use embedded_graphics::text::Baseline;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10, ascii::FONT_6X13},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::mode::DisplayConfigAsync;
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};

pub async fn init<'a>(mut i2c: I2c<'a, Async>) {
    let i2c = I2CDisplayInterface::new(&mut i2c);

    let mut display = Ssd1306Async::new(i2c, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let large_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X13)
        .text_color(BinaryColor::On)
        .build();

    display.init().await.unwrap();
    display.clear_buffer();

    Text::with_baseline("Status", Point::new(10, 0), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    Text::with_baseline(
        "Initialized",
        Point::new(10, 16),
        large_text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    display.flush().await.unwrap();
}
