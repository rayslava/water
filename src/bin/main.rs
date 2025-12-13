#![no_std]
#![no_main]

// use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock, interrupt::software::SoftwareInterruptControl, rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;
use water::appcore::start_appcore;
use water::display::{display_task, update_status};
use water::io::gpio::{
    adc_task, btn_init, compressor_init, get_battery_value, get_sensor_value, led_init,
};
use water::io::led::{HEARTBEAT_DEFAULT, heartbeat, set_heartbeat};
use water::io::rtc;
use water::io::wifi::wifi_hw_init;
use water::net::mqtt::mqtt_task;
use water::net::ntp::{NtpClient, ntp_task};
use water::net::stack::{init_net, wait_for_ip, wait_for_link};
use water::time::{now, set_last_watered};
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let rng = Rng::new();
    let wifi_timer = TimerGroup::new(peripherals.TIMG0).timer0;

    let timer_group1 = TimerGroup::new(peripherals.TIMG1);
    let embassy_timer = timer_group1.timer0;
    let watchdog = timer_group1.wdt;

    match water::watchdog::init_watchdog(watchdog) {
        Ok(()) => println!("Watchdog initialized successfully"),
        Err(e) => println!("Failed to initialize watchdog: {:?}", e),
    }

    esp_rtos::start(embassy_timer);

    // Initialize software interrupts for second core
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    let led = led_init(peripherals.GPIO2).await;
    let mut compressor = compressor_init(peripherals.GPIO25).await;
    let button = btn_init(peripherals.GPIO0).await;

    rtc::init(peripherals.LPWR).await;

    update_status("App core starting").await.unwrap();

    start_appcore(
        peripherals.CPU_CTRL,
        software_interrupt.software_interrupt0,
        software_interrupt.software_interrupt1,
        {
            let led_pin = led;
            move |spawner| {
                spawner.spawn(heartbeat(led_pin)).ok();
                spawner
                    .spawn(display_task(
                        peripherals.I2C0,
                        peripherals.GPIO21,
                        peripherals.GPIO22,
                    ))
                    .ok();
                spawner
                    .spawn(adc_task(
                        peripherals.GPIO36,
                        peripherals.GPIO34,
                        peripherals.ADC1,
                    ))
                    .ok();
            }
        },
    )
    .unwrap();

    update_status("WiFi init").await.unwrap();

    let wifi = wifi_hw_init(wifi_timer, rng, peripherals.WIFI, &spawner)
        .await
        .unwrap();

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let stack = init_net(wifi, seed, &spawner).await.unwrap();

    wait_for_link(stack).await;
    wait_for_ip(stack).await;
    set_heartbeat(HEARTBEAT_DEFAULT);

    let ntp = NtpClient::new(stack);
    spawner.spawn(ntp_task(ntp)).ok();
    spawner.spawn(mqtt_task(rng, stack)).ok();

    loop {
        let sens_val = get_sensor_value().await;
        let bat_val = get_battery_value().await;
        println!("Sensor: {}, Battery: {}", sens_val, bat_val);

        if button.is_low() {
            println!("Compressor ON");
            set_last_watered(now().await.unwrap()).await;
            compressor.set_high();
        } else {
            compressor.set_low();
        }
        Timer::after(Duration::from_millis(2000)).await;

        // We're still alive
        water::watchdog::feed_watchdog();
    }
}
