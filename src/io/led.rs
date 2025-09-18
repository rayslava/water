use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::gpio::Output;

#[embassy_executor::task]
pub async fn heartbeat(
    mut led: Output<'static>,
    control: &'static Signal<CriticalSectionRawMutex, bool>,
) {
    loop {
        if control.wait().await {
            led.set_low();
        } else {
            led.set_high();
        }
    }
}
