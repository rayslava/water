use crate::io::gpio::{get_battery_value, get_sensor_value};

/// Returns the charge level in percent
///
/// The computations are mostly random guesses but seem to show some value.
pub async fn charge_level() -> u32 {
    let adc_val = get_battery_value().await;

    // Approximate values:
    // 4000 - 0%
    // 1200 - 100%
    // 850 - charging
    (4000u32.saturating_sub(adc_val as u32)) * 100 / (4000 - 1200)
}

/// Returns the level moisture in percent
///
/// The computations are mostly random guesses but seem to show some value.
pub async fn humidity_level() -> u32 {
    let adc_val = get_sensor_value().await;

    // Approximate values:
    // 2950 - dry air
    // 3700 - water
    (adc_val.saturating_sub(2950) as u32) * 100 / (3700 - 2950)
}
