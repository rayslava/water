use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

static LOW_HUMIDITY_LIMIT: Mutex<CriticalSectionRawMutex, u16> = Mutex::new(10);

pub async fn get_low_humidity_limit() -> u16 {
    *LOW_HUMIDITY_LIMIT.lock().await
}

pub async fn set_low_humidity_limit(lim: u16) {
    *LOW_HUMIDITY_LIMIT.lock().await = lim;
}
