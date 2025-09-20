use core::fmt::Write;
use embedded_graphics::{
    Drawable,
    image::{Image, ImageRaw},
    mono_font::{MonoFont, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use heapless::String;

use crate::{error::UIError, io::wifi::is_wifi_connected, power::charge_level, time::localtime};

use super::{
    DISPLAY_HEIGHT, DISPLAY_WIDTH, STATUS_BAR_HEIGHT, STATUS_LINE_HEIGHT, STATUS_LINE_TOP,
};

pub(crate) async fn draw_markup(
    target: &mut impl DrawTarget<Color = BinaryColor>,
) -> Result<(), UIError> {
    // The status bar separation
    Ok(Line::new(
        Point::new(0, STATUS_LINE_TOP),
        Point::new(DISPLAY_WIDTH, STATUS_LINE_TOP),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}

const CLOCK_FONT: MonoFont<'_> = embedded_graphics::mono_font::ascii::FONT_9X15;
const CLOCK_FONT_WIDTH: i32 = CLOCK_FONT.character_size.width as i32;

// Rightmost position is time
const TIME_WIDTH: i32 = CLOCK_FONT_WIDTH * 5; // HH:MM

const BATTERY_WIDTH: u32 = 32;
const BATTERY_X: i32 = DISPLAY_WIDTH - TIME_WIDTH - BATTERY_WIDTH as i32 - 3;
const BATTERY_HEIGHT: u32 = STATUS_BAR_HEIGHT as u32 - 1;

async fn draw_battery(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Rectangle::new(
        Point::new(BATTERY_X, 1),
        Size::new(BATTERY_WIDTH - 2, BATTERY_HEIGHT),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?;
    Rectangle::new(
        Point::new(
            BATTERY_X + BATTERY_WIDTH as i32 - 2,
            BATTERY_HEIGHT as i32 / 4 + 2,
        ),
        Size::new(3, BATTERY_HEIGHT / 2),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?;

    let battery_level = (BATTERY_WIDTH - 2) * charge_level().await / 100;

    if battery_level > 1 {
        Ok(Rectangle::new(
            Point::new(BATTERY_X, 1),
            Size::new(battery_level, BATTERY_HEIGHT),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(&mut *target)
        .map_err(|_| UIError::DrawError)?)
    } else {
        let text_style = MonoTextStyleBuilder::new()
            .font(&CLOCK_FONT)
            .text_color(BinaryColor::On)
            .build();

        Text::with_baseline(
            "LOW",
            Point::new(BATTERY_X + 1, 1),
            text_style,
            Baseline::Top,
        )
        .draw(&mut *target)
        .map_err(|_| UIError::DrawError)?;
        Ok(())
    }
}

async fn draw_clock(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<Point, UIError> {
    let text_style = MonoTextStyleBuilder::new()
        .font(&CLOCK_FONT)
        .text_color(BinaryColor::On)
        .build();
    let mut timestring: String<5> = String::new();
    if let Ok(time) = localtime().await {
        write!(timestring, "{:02}:{:02}", time.hour(), time.minute()).ok();
    } else {
        write!(timestring, "--:--").ok();
    }
    Ok(Text::with_baseline(
        &timestring,
        Point::new(DISPLAY_WIDTH - TIME_WIDTH, 1),
        text_style,
        Baseline::Top,
    )
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}

async fn draw_wifi(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    const WIFI_IMAGE: ImageRaw<BinaryColor> =
        ImageRaw::new(include_bytes!("../../icons/wifi.raw"), 16);
    const NOWIFI_IMAGE: ImageRaw<BinaryColor> =
        ImageRaw::new(include_bytes!("../../icons/nowifi.raw"), 16);
    let image = if is_wifi_connected().await {
        Image::new(&WIFI_IMAGE, Point::zero())
    } else {
        Image::new(&NOWIFI_IMAGE, Point::zero())
    };
    image.draw(&mut *target).map_err(|_| UIError::DrawError)?;
    Ok(())
}

async fn draw_net(_target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Ok(())
}

async fn draw_main(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Ok(Rectangle::new(
        Point::new(0, STATUS_BAR_HEIGHT),
        Size::new(
            DISPLAY_WIDTH as u32,
            (DISPLAY_HEIGHT - STATUS_BAR_HEIGHT - STATUS_LINE_HEIGHT) as u32,
        ),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}

pub(crate) async fn draw_status_bar(
    target: &mut impl DrawTarget<Color = BinaryColor>,
) -> Result<(), UIError> {
    draw_net(target).await?;
    draw_wifi(target).await?;
    draw_clock(target).await?;
    draw_battery(target).await?;
    draw_main(target).await?;
    Ok(())
}
