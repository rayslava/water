//! Embassy DHCP Example
//!
//!
//! Set SSID and PASSWORD env variable before running this example.
//!
//! This gets an ip address via DHCP then performs an HTTP get request to some
//! "random" server
#![no_std]
#![no_main]

use core::net::Ipv4Addr;

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use water::appcore::start_appcore;
use water::display;
use water::io::gpio::led_init;
use water::io::led::{HEARTBEAT_DEFAULT, HEARTBEAT_NET_AWAIT, heartbeat, set_heartbeat};
use water::io::wifi::wifi_hw_init;
use water::net::stack::{init_net, wait_for_link};
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    println!("Init display");
    let display = display::init(peripherals.I2C0, peripherals.GPIO21, peripherals.GPIO22)
        .await
        .unwrap();

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    display.update_status("LED init").await.unwrap();
    display.clear().await.unwrap();
    let led = led_init(peripherals.GPIO2).await;

    display.update_status("CPU 2 init").await.unwrap();
    display.clear().await.unwrap();

    // Initialize app core with LED heartbeat task
    let _appcore_guard = start_appcore(peripherals.CPU_CTRL, {
        let led_pin = led;
        move |spawner| {
            spawner.spawn(heartbeat(led_pin)).ok();
        }
    })
    .unwrap();

    display.update_status("WiFi init").await.unwrap();
    display.clear().await.unwrap();

    let wifi = wifi_hw_init(timg0.timer0, rng, peripherals.WIFI, &spawner)
        .await
        .unwrap();

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let stack = init_net(wifi, seed, &spawner).await.unwrap();

    set_heartbeat(HEARTBEAT_NET_AWAIT);
    display.update_status("Connecting").await.unwrap();
    wait_for_link(stack).await;

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    set_heartbeat(HEARTBEAT_DEFAULT);

    loop {
        Timer::after(Duration::from_millis(1_000)).await;
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        let remote_endpoint = (Ipv4Addr::new(142, 250, 185, 115), 80);
        println!("connecting...");
        let r = socket.connect(remote_endpoint).await;
        if let Err(e) = r {
            println!("connect error: {:?}", e);
            continue;
        }
        println!("connected!");
        let mut buf = [0; 1024];
        loop {
            use embedded_io_async::Write;
            let r = socket
                .write_all(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
                .await;
            if let Err(e) = r {
                println!("write error: {:?}", e);
                break;
            }
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };
            println!("{}", core::str::from_utf8(&buf[..n]).unwrap());
        }
        Timer::after(Duration::from_millis(3000)).await;
    }
}
