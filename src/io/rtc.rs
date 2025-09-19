use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::peripherals::LPWR;
use esp_hal::rtc_cntl::Rtc;

use crate::error::SysError;

static GLOBAL_RTC: Mutex<CriticalSectionRawMutex, Option<Rtc>> = Mutex::new(None);

pub async fn get_time() -> Result<u64, SysError> {
    let rtc = GLOBAL_RTC.lock().await;
    if let Some(rtc) = rtc.as_ref() {
        return Ok(rtc.current_time_us() as u64);
    } else {
        Err(SysError::TimerSetup)
    }
}

pub async fn set_time(stamp: u64) -> Result<(), SysError> {
    let rtc = GLOBAL_RTC.lock().await;
    if let Some(rtc) = rtc.as_ref() {
        Ok(rtc.set_current_time_us(stamp))
    } else {
        Err(SysError::TimerSetup)
    }
}

pub async fn init(peripheral: LPWR<'static>) {
    let rtc = Rtc::new(peripheral);

    GLOBAL_RTC.lock().await.replace(rtc);
}
