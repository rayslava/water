use embedded_graphics::{
    Drawable,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};

use crate::error::UIError;

use super::{DISPLAY_WIDTH, STATUS_LINE_TOP};

pub async fn draw_markup(target: &mut impl DrawTarget<Color = BinaryColor>) -> Result<(), UIError> {
    // The status bar separation
    Ok(Line::new(
        Point::new(0, STATUS_LINE_TOP),
        Point::new(DISPLAY_WIDTH, STATUS_LINE_TOP),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(&mut *target)
    .map_err(|_| UIError::DrawError)?)
}
