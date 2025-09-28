use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{ADC1, GPIO0, GPIO2, GPIO25, GPIO34, GPIO36},
};

pub async fn led_init(gpio: GPIO2<'static>) -> Output<'static> {
    Output::new(gpio, Level::Low, OutputConfig::default()) // Start with LED off
}

pub async fn compressor_init(gpio: GPIO25<'static>) -> Output<'static> {
    Output::new(gpio, Level::Low, OutputConfig::default())
}

pub async fn btn_init(gpio: GPIO0<'static>) -> Input<'static> {
    Input::new(gpio, InputConfig::default().with_pull(Pull::Up))
}

type MainAdc = ADC1<'static>;
type BatPin = GPIO36<'static>;
type SensPin = GPIO34<'static>;

static BAT_VAL: Mutex<CriticalSectionRawMutex, u16> = Mutex::new(0);
static SENSOR_VAL: Mutex<CriticalSectionRawMutex, u16> = Mutex::new(0);

//const ADC_REFRESH_TIME: Duration = Duration::from_secs(60);

const ADC_REFRESH_TIME: Duration = Duration::from_millis(800);

#[embassy_executor::task]
pub async fn adc_task(battery_pin: BatPin, sensor_pin: SensPin, adc: MainAdc) {
    let mut adc1_config = AdcConfig::new();
    let mut pin_bat = adc1_config.enable_pin(battery_pin, Attenuation::_11dB);
    let mut pin_sensor = adc1_config.enable_pin(sensor_pin, Attenuation::_11dB);
    let mut adc = Adc::new(adc, adc1_config);
    loop {
        let bat_value: u16 = nb::block!(adc.read_oneshot(&mut pin_bat)).unwrap_or(4096);
        let sens_value: u16 = nb::block!(adc.read_oneshot(&mut pin_sensor)).unwrap_or(4096);

        *BAT_VAL.lock().await = bat_value;
        *SENSOR_VAL.lock().await = sens_value;
        Timer::after(ADC_REFRESH_TIME).await;
    }
}

pub async fn get_battery_value() -> u16 {
    *BAT_VAL.lock().await
}

pub async fn get_sensor_value() -> u16 {
    *SENSOR_VAL.lock().await
}
