use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embassy_time::{Duration, Instant};
use esp_println::println;

// Health check timeouts (in milliseconds)
const WIFI_HEALTH_TIMEOUT: Duration = Duration::from_secs(120);
const MQTT_HEALTH_TIMEOUT: Duration = Duration::from_secs(240);
const DISPLAY_HEALTH_TIMEOUT: Duration = Duration::from_secs(30);
const ADC_HEALTH_TIMEOUT: Duration = Duration::from_secs(60);

// Health monitoring state for each subsystem
static WIFI_LAST_HEARTBEAT: AtomicU32 = AtomicU32::new(0);
static MQTT_LAST_HEARTBEAT: AtomicU32 = AtomicU32::new(0);
static DISPLAY_LAST_HEARTBEAT: AtomicU32 = AtomicU32::new(0);
static ADC_LAST_HEARTBEAT: AtomicU32 = AtomicU32::new(0);

static WIFI_HEALTHY: AtomicBool = AtomicBool::new(true);
static MQTT_HEALTHY: AtomicBool = AtomicBool::new(true);
static DISPLAY_HEALTHY: AtomicBool = AtomicBool::new(true);
static ADC_HEALTHY: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Copy, Clone)]
pub enum Subsystem {
    Wifi,
    Mqtt,
    Display,
    Adc,
}

impl Subsystem {
    pub fn name(&self) -> &'static str {
        match self {
            Subsystem::Wifi => "WiFi",
            Subsystem::Mqtt => "MQTT",
            Subsystem::Display => "Display",
            Subsystem::Adc => "ADC",
        }
    }
}

pub fn record_heartbeat(subsystem: Subsystem) {
    let now: u32 = Instant::now().as_millis() as u32;

    match subsystem {
        Subsystem::Wifi => {
            WIFI_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
            WIFI_HEALTHY.store(true, Ordering::SeqCst);
        }
        Subsystem::Mqtt => {
            MQTT_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
            MQTT_HEALTHY.store(true, Ordering::SeqCst);
        }
        Subsystem::Display => {
            DISPLAY_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
            DISPLAY_HEALTHY.store(true, Ordering::SeqCst);
        }
        Subsystem::Adc => {
            ADC_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
            ADC_HEALTHY.store(true, Ordering::SeqCst);
        }
    }
}

pub fn is_subsystem_healthy(subsystem: Subsystem) -> bool {
    let now: u32 = Instant::now().as_millis() as u32;
    let (last_heartbeat, timeout, healthy_flag) = match subsystem {
        Subsystem::Wifi => (
            WIFI_LAST_HEARTBEAT.load(Ordering::SeqCst),
            WIFI_HEALTH_TIMEOUT.as_millis(),
            &WIFI_HEALTHY,
        ),
        Subsystem::Mqtt => (
            MQTT_LAST_HEARTBEAT.load(Ordering::SeqCst),
            MQTT_HEALTH_TIMEOUT.as_millis(),
            &MQTT_HEALTHY,
        ),
        Subsystem::Display => (
            DISPLAY_LAST_HEARTBEAT.load(Ordering::SeqCst),
            DISPLAY_HEALTH_TIMEOUT.as_millis(),
            &DISPLAY_HEALTHY,
        ),
        Subsystem::Adc => (
            ADC_LAST_HEARTBEAT.load(Ordering::SeqCst),
            ADC_HEALTH_TIMEOUT.as_millis(),
            &ADC_HEALTHY,
        ),
    };

    // If never received a heartbeat, consider it healthy initially
    if last_heartbeat == 0 {
        return true;
    }

    let is_healthy = now.wrapping_sub(last_heartbeat) < (timeout as u32);

    let was_healthy = healthy_flag.load(Ordering::SeqCst);
    if is_healthy != was_healthy {
        healthy_flag.store(is_healthy, Ordering::SeqCst);
        if is_healthy {
            println!("{} subsystem recovered", subsystem.name());
        } else {
            println!(
                "WARNING: {} subsystem unhealthy - no heartbeat for {}ms",
                subsystem.name(),
                now.wrapping_sub(last_heartbeat)
            );
        }
    }

    is_healthy
}

pub fn is_system_healthy() -> bool {
    let wifi_ok = is_subsystem_healthy(Subsystem::Wifi);
    let mqtt_ok = is_subsystem_healthy(Subsystem::Mqtt);
    let display_ok = is_subsystem_healthy(Subsystem::Display);
    let adc_ok = is_subsystem_healthy(Subsystem::Adc);

    wifi_ok && mqtt_ok && display_ok && adc_ok
}

pub fn get_health_status() -> (bool, bool, bool, bool) {
    (
        is_subsystem_healthy(Subsystem::Wifi),
        is_subsystem_healthy(Subsystem::Mqtt),
        is_subsystem_healthy(Subsystem::Display),
        is_subsystem_healthy(Subsystem::Adc),
    )
}

pub fn init_health_monitoring() {
    let now: u32 = Instant::now().as_millis() as u32;

    // Initialize all heartbeats to current time
    WIFI_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
    MQTT_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
    DISPLAY_LAST_HEARTBEAT.store(now, Ordering::SeqCst);
    ADC_LAST_HEARTBEAT.store(now, Ordering::SeqCst);

    println!("Health monitoring initialized for all subsystems");
}
