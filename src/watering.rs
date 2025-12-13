use crate::power::humidity_level;
use crate::time::{now, set_last_watered};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, Output};
use esp_println::println;

static LOW_HUMIDITY_LIMIT: Mutex<CriticalSectionRawMutex, u16> = Mutex::new(10);

pub async fn get_low_humidity_limit() -> u16 {
    *LOW_HUMIDITY_LIMIT.lock().await
}

pub async fn set_low_humidity_limit(lim: u16) {
    LOW_HUMIDITY_LIMIT.lock().await.clone_from(&lim);
}

#[embassy_executor::task]
pub async fn watering_task(mut compressor: Output<'static>, button: Option<Input<'static>>) {
    // Tunables
    let max_on_time = Duration::from_secs(30);
    let poll_idle = Duration::from_millis(800);
    let poll_active = Duration::from_millis(250);
    let hysteresis: u32 = 2; // percent
    let consecutive_triggers: u8 = 2; // debounce before starting cycle
    let consecutive_clear: u8 = 3; // debounce before stopping early
    let cooldown = Duration::from_secs(3);

    let mut below_count: u8 = 0;
    let mut clear_count: u8 = 0;

    loop {
        // Manual override: keep compressor ON while button held
        if let Some(ref btn) = button
            && btn.is_low()
        {
            compressor.set_high();
            Timer::after(poll_active).await;
            continue;
        }

        // Idle monitoring loop
        let hum = humidity_level().await; // percent
        let limit = get_low_humidity_limit().await as u32;

        if hum < limit {
            below_count = below_count.saturating_add(1);
        } else {
            below_count = 0;
        }

        if below_count >= consecutive_triggers {
            println!("Watering: humidity {}% < limit {}% → start", hum, limit);
            // Record watering start
            if let Ok(ts) = now().await {
                set_last_watered(ts).await;
            }

            compressor.set_high();
            let mut elapsed_ms: u32 = 0;
            clear_count = 0;
            while elapsed_ms < max_on_time.as_millis() as u32 {
                // If manual override pressed during watering, remain ON but continue counting time
                if let Some(ref btn) = button
                    && btn.is_low()
                {
                    Timer::after(poll_active).await;
                    elapsed_ms = elapsed_ms.saturating_add(poll_active.as_millis() as u32);
                    continue;
                }

                let hum_now = humidity_level().await;
                if hum_now >= limit.saturating_add(hysteresis) {
                    clear_count = clear_count.saturating_add(1);
                } else {
                    clear_count = 0;
                }

                if clear_count >= consecutive_clear {
                    println!(
                        "Watering: early stop at {}% (≥ {}% + {}%)",
                        hum_now, limit, hysteresis
                    );
                    break;
                }

                Timer::after(poll_active).await;
                elapsed_ms = elapsed_ms.saturating_add(poll_active.as_millis() as u32);
            }

            compressor.set_low();
            println!("Watering: cycle complete");
            below_count = 0;
            Timer::after(cooldown).await;
        } else {
            Timer::after(poll_idle).await;
        }
    }
}
