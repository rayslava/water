use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::GPIO2,
};
use static_cell::StaticCell;

pub struct LedInitResult {
    pub led: Output<'static>,
    pub control_signal: &'static Signal<CriticalSectionRawMutex, bool>,
}

pub async fn led_init(gpio2: GPIO2<'static>) -> LedInitResult {
    let led = Output::new(gpio2, Level::Low, OutputConfig::default());

    static LED_CTRL: StaticCell<Signal<CriticalSectionRawMutex, bool>> = StaticCell::new();
    let led_ctrl_signal = LED_CTRL.init(Signal::new());

    LedInitResult {
        led,
        control_signal: led_ctrl_signal,
    }
}
