use crate::display::{STATUS_LEN, update_status};
use crate::error::SysError;
use crate::io::led::{HEARTBEAT_DEFAULT, HEARTBEAT_NET_AWAIT, set_heartbeat};
use core::fmt::Write;
// use alloc::string::ToString;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::{peripherals::WIFI, rng::Rng, timer::timg::Timer as HalTimer};
use esp_println::println;
use esp_radio::wifi::{ClientConfig, ModeConfig, ScanConfig, WifiEvent, WifiStaState};
use esp_radio::{
    Controller, init,
    wifi::{Config, WifiController, WifiDevice, new},
};
use heapless::String;
use static_cell::StaticCell;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

const RECONNECT_DELAY: Duration = Duration::from_millis(5000);
static WIFI_CONNECTED: Mutex<CriticalSectionRawMutex, bool> = Mutex::new(false);

pub async fn wifi_hw_init(
    _timer: HalTimer<'static>,
    _rng: Rng,
    wifi_peripheral: WIFI<'static>,
    spawner: &Spawner,
) -> Result<WifiDevice<'static>, SysError> {
    // Initialize ESP WiFi hardware
    // Note: timer and rng parameters are preserved for API compatibility
    // but esp-radio 0.17 handles timing and randomness internally
    let esp_wifi_ctrl = {
        static ESP_WIFI_CTRL: StaticCell<Controller> = StaticCell::new();
        ESP_WIFI_CTRL.init(init()?)
    };

    let config = Config::default();
    let (controller, interfaces) = new(esp_wifi_ctrl, wifi_peripheral, config)?;

    spawner.spawn(maintain_connection(controller))?;

    Ok(interfaces.sta)
}

pub async fn is_wifi_connected() -> bool {
    *WIFI_CONNECTED.lock().await
}

// We have to run this function in the background to keep the wifi on
#[embassy_executor::task]
async fn maintain_connection(mut controller: WifiController<'static>) {
    loop {
        if esp_radio::wifi::sta_state() == WifiStaState::Connected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            update_status("WiFi disconnected").await.ok();
            set_heartbeat(HEARTBEAT_NET_AWAIT);
            WIFI_CONNECTED.lock().await.clone_from(&false);
            Timer::after(RECONNECT_DELAY).await
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            update_status("Starting WiFi").await.ok();
            controller.start_async().await.unwrap();
            update_status("WiFi scan").await.ok();
            let scan_config = ScanConfig::default();
            let result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();
            for ap in result.iter().take(5) {
                // Limit to first 5 APs to avoid too much output
                println!("Found AP: {:?}", ap.ssid);
            }
        }

        update_status("Connecting to WiFi").await.ok();

        match controller.connect_async().await {
            Ok(_) => {
                update_status("Wifi connected!").await.ok();
                set_heartbeat(HEARTBEAT_DEFAULT);
                WIFI_CONNECTED.lock().await.clone_from(&true);
            }
            Err(e) => {
                set_heartbeat(HEARTBEAT_NET_AWAIT);
                let mut errstring: String<STATUS_LEN> = String::new();
                write!(errstring, "WiFi fail: {:?}", e).ok();
                update_status(&errstring).await.ok();
                Timer::after(RECONNECT_DELAY).await
            }
        }
    }
}
