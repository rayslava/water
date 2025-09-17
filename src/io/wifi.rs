use crate::error::HwError;
use esp_hal::{peripherals::WIFI, rng::Rng, timer::timg::Timer};
use esp_wifi::{
    EspWifiController, init,
    wifi::{WifiController, WifiDevice, new},
};
use static_cell::StaticCell;

pub struct WifiHwInitResult {
    pub controller: WifiController<'static>,
    pub interface: WifiDevice<'static>,
}

pub async fn wifi_hw_init(
    timer: Timer<'static>,
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
