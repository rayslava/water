use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

pub const HEARTBEAT_DEFAULT: Duration = Duration::from_millis(5000);
pub const HEARTBEAT_NET_AWAIT: Duration = Duration::from_millis(1000);
pub const HEARTBEAT_INIT: Duration = Duration::from_millis(500);
const HEARTBEAT_BLINK_TIME: Duration = Duration::from_millis(100);

static DURATION_SIGNAL: Signal<CriticalSectionRawMutex, Duration> = Signal::new();

pub fn set_heartbeat(duration: Duration) {
    DURATION_SIGNAL.signal(duration);
}

#[embassy_executor::task]
pub async fn heartbeat(mut led: Output<'static>) {
    let mut duration = HEARTBEAT_INIT;

    loop {
        if let Some(new_duration) = DURATION_SIGNAL.try_take() {
            duration = new_duration;
        }

        Timer::after(duration).await;
        led.set_high();
        Timer::after(HEARTBEAT_BLINK_TIME).await;
        led.set_low();
    }
}
