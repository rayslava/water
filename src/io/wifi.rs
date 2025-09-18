use crate::error::HwError;
use embassy_time::{Duration, Timer};
use esp_hal::{peripherals::WIFI, rng::Rng, timer::timg::Timer as HalTimer};
use esp_println::println;
use esp_wifi::wifi::{ClientConfiguration, Configuration, WifiEvent, WifiState};
use esp_wifi::{
    EspWifiController, init,
    wifi::{WifiController, WifiDevice, new},
};
use static_cell::StaticCell;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub struct WifiHwInitResult {
    pub controller: WifiController<'static>,
    pub interface: WifiDevice<'static>,
}

pub async fn wifi_hw_init(
    timer: HalTimer<'static>,
    rng: Rng,
    wifi_peripheral: WIFI<'static>,
) -> Result<WifiHwInitResult, HwError> {
    // Initialize ESP WiFi hardware
    let esp_wifi_ctrl = {
        static ESP_WIFI_CTRL: StaticCell<EspWifiController> = StaticCell::new();
        ESP_WIFI_CTRL.init(init(timer, rng)?)
    };

    let (controller, interfaces) = new(esp_wifi_ctrl, wifi_peripheral)?;

    Ok(WifiHwInitResult {
        controller,
        interface: interfaces.sta,
    })
}

#[embassy_executor::task]
pub async fn maintain_connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");

            println!("Scan");
            let result = controller.scan_n_async(10).await.unwrap();
            for ap in result {
                println!("{:?}", ap);
            }
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}
