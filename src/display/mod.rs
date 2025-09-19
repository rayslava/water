use crate::error::{ConversionError, HwError, UIError};
use crate::io::i2c::display_i2c_init;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::Baseline;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_9X15},
    pixelcolor::BinaryColor,
    text::Text,
};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use esp_hal::peripherals::{GPIO21, GPIO22, I2C0};
use ssd1306::mode::{BufferedGraphicsModeAsync, DisplayConfigAsync};
use ssd1306::prelude::I2CInterface;
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use static_cell::StaticCell;
mod gui;

const DISPLAY_WIDTH: i32 = 128;
const DISPLAY_HEIGHT: i32 = 64;

const FONT_HEIGHT: i32 = 15;
const FONT_WIDTH: i32 = 15;

const STATUS_LINE_TOP: i32 = DISPLAY_HEIGHT - FONT_HEIGHT - 1;

const STATUS_LEN: usize = DISPLAY_WIDTH as usize / FONT_WIDTH as usize;

pub type Display<'a> = Ssd1306Async<
    I2CInterface<&'a mut I2c<'a, Async>>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

static STATUS: Mutex<CriticalSectionRawMutex, [u8; STATUS_LEN]> = Mutex::new([0u8; STATUS_LEN]);

pub struct DisplayHandle {
    display_mutex: &'static Mutex<CriticalSectionRawMutex, Display<'static>>,
}

impl DisplayHandle {
    async fn markup(&self) -> Result<(), UIError> {
        let mut display = self.display_mutex.lock().await;
        Ok(gui::draw_markup(&mut *display).await?)
    }

    async fn print_status(&self) -> Result<(), UIError> {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X15)
            .text_color(BinaryColor::On)
            .build();

        let mut display = self.display_mutex.lock().await;
        let status = STATUS.lock().await;

        // Convert &[u8] to &str
        let status_str = core::str::from_utf8(&*status)?;

        Text::with_baseline(
            status_str,
            Point::new(0, DISPLAY_HEIGHT - FONT_HEIGHT),
            text_style,
            Baseline::Top,
        )
        .draw(&mut *display)?;
        Ok(())
    }

    async fn clear_buffer(&self) {
        let mut display = self.display_mutex.lock().await;
        display.clear_buffer()
    }

    async fn flush(&self) -> Result<(), HwError> {
        let mut display = self.display_mutex.lock().await;
        Ok(display.flush().await?)
    }

    pub async fn clear(&self) -> Result<(), UIError> {
        self.clear_buffer().await;
        self.markup().await?;
        self.print_status().await?;
        self.flush().await?;
        Ok(())
    }
}

pub async fn init(
    i2c_peripheral: I2C0<'static>,
    sda: GPIO21<'static>,
    scl: GPIO22<'static>,
) -> Result<DisplayHandle, HwError> {
    static I2C_CELL: StaticCell<I2c<'static, Async>> = StaticCell::new();
    let i2c = I2C_CELL.init(display_i2c_init(i2c_peripheral, sda, scl).await?);

    let i2c_interface = I2CDisplayInterface::new(i2c);

    let mut display = Ssd1306Async::new(i2c_interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().await?;
    display.clear_buffer();
    display.flush().await?;

    static DISPLAY_MUTEX: StaticCell<Mutex<CriticalSectionRawMutex, Display<'static>>> =
        StaticCell::new();
    let display_mutex = DISPLAY_MUTEX.init(Mutex::new(display));
    
    Ok(DisplayHandle { display_mutex })
}

pub async fn update_status(new_status: &str) -> Result<(), ConversionError> {
    let new_status_buf: &[u8] = new_status.as_bytes();
    let copy_len = (new_status_buf.len()).min(STATUS_LEN - 1);
    let mut status: [u8; STATUS_LEN] = *STATUS.lock().await;
    status.fill(0);
    status.copy_from_slice(&new_status_buf[..copy_len]);
    Ok(())
}

const DISPLAY_REFRESH_TIME: Duration = Duration::from_millis(500);

#[embassy_executor::task]
pub async fn display_task(
    i2c_peripheral: I2C0<'static>,
    sda: GPIO21<'static>,
    scl: GPIO22<'static>,
) {
    let display = init(i2c_peripheral, sda, scl).await.unwrap();
    loop {
        display.clear().await.unwrap();
        Timer::after(DISPLAY_REFRESH_TIME).await;
    }
}
