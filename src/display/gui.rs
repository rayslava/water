use embedded_graphics::{
    Drawable,
    mono_font::{MonoFont, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

use crate::error::UIError;

use super::{DISPLAY_WIDTH, FONT_WIDTH, STATUS_BAR_HEIGHT, STATUS_LINE_TOP};

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
const BATTERY_X: i32 = DISPLAY_WIDTH - TIME_WIDTH - BATTERY_WIDTH as i32 - 2;
const BATTERY_HEIGHT: u32 = STATUS_BAR_HEIGHT as u32 - 1;

async fn draw_battery(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Rectangle::new(
        Point::new(BATTERY_X, 1),
        Size::new(BATTERY_WIDTH - 2, BATTERY_HEIGHT),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?;
    Ok(Rectangle::new(
        Point::new(
            BATTERY_X + BATTERY_WIDTH as i32 - 2,
            BATTERY_HEIGHT as i32 / 4 + 2,
        ),
        Size::new(3, BATTERY_HEIGHT / 2),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}

async fn draw_clock(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<Point, UIError> {
    let text_style = MonoTextStyleBuilder::new()
        .font(&CLOCK_FONT)
        .text_color(BinaryColor::On)
        .build();
    Ok(Text::with_baseline(
        "00:00",
        Point::new(DISPLAY_WIDTH - TIME_WIDTH, 1),
        text_style,
        Baseline::Top,
    )
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}

async fn draw_wifi(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Ok(())
}

async fn draw_net(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Ok(())
}

pub(crate) async fn draw_status_bar(
    target: &mut impl DrawTarget<Color = BinaryColor>,
) -> Result<(), UIError> {
    draw_net(target).await?;
    draw_wifi(target).await?;
    draw_clock(target).await?;
    draw_battery(target).await?;
    Ok(())
}
