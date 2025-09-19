use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::GPIO2,
};

pub async fn led_init(gpio2: GPIO2<'static>) -> Output<'static> {
    Output::new(gpio2, Level::Low, OutputConfig::default()) // Start with LED off
}
