use embedded_graphics::{
    Drawable,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
};

use crate::error::UIError;

use super::{DISPLAY_WIDTH, STATUS_BAR_HEIGHT, STATUS_LINE_TOP};

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

const BATTERY_X: i32 = 100;
const BATTERY_WIDTH: u32 = 16;

async fn draw_battery(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Ok(Rectangle::new(
        Point::new(BATTERY_X, 1),
        Size::new(
            BATTERY_X as u32 + BATTERY_WIDTH,
            STATUS_BAR_HEIGHT as u32 - 1,
        ),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}

async fn draw_clock(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    Ok(())
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
