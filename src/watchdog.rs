use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use critical_section;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::peripherals::TIMG1;
use esp_hal::time::Duration;
use esp_hal::timer::timg::{MwdtStage, Wdt};
use esp_println::println;

use crate::error::SysError;

const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(60);

static WATCHDOG_ENABLED: AtomicBool = AtomicBool::new(false);
static FEED_COUNT: AtomicU32 = AtomicU32::new(0);
static WATCHDOG_INITIALIZED: AtomicBool = AtomicBool::new(false);

static GLOBAL_WDT: Mutex<CriticalSectionRawMutex, Option<Wdt<TIMG1<'static>>>> = Mutex::new(None);

pub fn init_watchdog(mut wdt: Wdt<TIMG1<'static>>) -> Result<(), SysError> {
    // Configure watchdog timeout (Stage 0 is the main timeout stage)
    wdt.set_timeout(MwdtStage::Stage0, WATCHDOG_TIMEOUT);

    // Enable the watchdog
    wdt.enable();

    println!("Watchdog initialized with 60s timeout on TIMG1");

    // Store the watchdog in global mutex
    critical_section::with(|_| {
        // We can't use async in this sync context, so we use critical_section for atomic access
        let mut wdt_guard = GLOBAL_WDT.try_lock().unwrap();
        *wdt_guard = Some(wdt);
    });

    // Set global state
    WATCHDOG_ENABLED.store(true, Ordering::SeqCst);
    WATCHDOG_INITIALIZED.store(true, Ordering::SeqCst);

    Ok(())
}

pub fn feed_watchdog() {
    if WATCHDOG_ENABLED.load(Ordering::SeqCst) {
        critical_section::with(|_| {
            if let Ok(mut wdt_guard) = GLOBAL_WDT.try_lock()
                && let Some(wdt) = wdt_guard.as_mut() {
                    wdt.feed();
                    FEED_COUNT.fetch_add(1, Ordering::SeqCst);
                }
        });
    }
}

pub fn is_watchdog_enabled() -> bool {
    WATCHDOG_ENABLED.load(Ordering::SeqCst) && WATCHDOG_INITIALIZED.load(Ordering::SeqCst)
}

pub fn get_watchdog_stats() -> (bool, u32) {
    let enabled = is_watchdog_enabled();
    let count = FEED_COUNT.load(Ordering::SeqCst);
    (enabled, count)
}

pub fn disable_watchdog() {
    critical_section::with(|_| {
        if let Ok(mut wdt_guard) = GLOBAL_WDT.try_lock()
            && let Some(wdt) = wdt_guard.as_mut() {
                wdt.disable();
                WATCHDOG_ENABLED.store(false, Ordering::SeqCst);
                println!("WARNING: Watchdog disabled");
            }
    });
}

/// Re-enable watchdog after temporary disable
pub fn enable_watchdog() {
    critical_section::with(|_| {
        if let Ok(mut wdt_guard) = GLOBAL_WDT.try_lock()
            && let Some(wdt) = wdt_guard.as_mut() {
                wdt.enable();
                WATCHDOG_ENABLED.store(true, Ordering::SeqCst);
                println!("Watchdog re-enabled");
            }
    });
}
