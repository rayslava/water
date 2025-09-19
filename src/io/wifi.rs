use crate::display::{STATUS_LEN, update_status};
use crate::error::SysError;
use crate::io::led::{HEARTBEAT_DEFAULT, HEARTBEAT_NET_AWAIT, set_heartbeat};
use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{peripherals::WIFI, rng::Rng, timer::timg::Timer as HalTimer};
use esp_println::println;
use esp_wifi::wifi::{ClientConfiguration, Configuration, WifiEvent, WifiState};
use esp_wifi::{
    EspWifiController, init,
    wifi::{WifiController, WifiDevice, new},
};
use heapless::String;
use static_cell::StaticCell;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

const RECONNECT_DELAY: Duration = Duration::from_millis(5000);

pub async fn wifi_hw_init(
    timer: HalTimer<'static>,
    rng: Rng,
    wifi_peripheral: WIFI<'static>,
    spawner: &Spawner,
) -> Result<WifiDevice<'static>, SysError> {
    // Initialize ESP WiFi hardware
    let esp_wifi_ctrl = {
        static ESP_WIFI_CTRL: StaticCell<EspWifiController> = StaticCell::new();
        ESP_WIFI_CTRL.init(init(timer, rng)?)
    };

    let (controller, interfaces) = new(esp_wifi_ctrl, wifi_peripheral)?;

    spawner.spawn(maintain_connection(controller))?;

    Ok(interfaces.sta)
}

// We have to run this function in the background to keep the wifi on
#[embassy_executor::task]
async fn maintain_connection(mut controller: WifiController<'static>) {
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            update_status("WiFi disconnected").await.ok();
            set_heartbeat(HEARTBEAT_NET_AWAIT);
            Timer::after(RECONNECT_DELAY).await
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            update_status("Starting WiFi").await.ok();
            controller.start_async().await.unwrap();
            update_status("WiFi scan").await.ok();
            let result = controller.scan_n_async(10).await.unwrap();
            for ap in result {
                println!("{:?}", ap);
            }
        }

        update_status("Connecting to WiFi").await.ok();

        match controller.connect_async().await {
            Ok(_) => {
                update_status("Wifi connected!").await.ok();
                set_heartbeat(HEARTBEAT_DEFAULT);
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
