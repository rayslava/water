//! Embassy DHCP Example
//!
//!
//! Set SSID and PASSWORD env variable before running this example.
//!
//! This gets an ip address via DHCP then performs an HTTP get request to some
//! "random" server
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use water::appcore::start_appcore;
use water::display::{display_task, update_status};
use water::io::gpio::led_init;
use water::io::led::{HEARTBEAT_DEFAULT, heartbeat, set_heartbeat};
use water::io::rtc;
use water::io::wifi::wifi_hw_init;
use water::net::ntp::NtpClient;
use water::net::stack::{init_net, wait_for_ip, wait_for_link};
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let mut rng = Rng::new(peripherals.RNG);
    let wifi_timer = TimerGroup::new(peripherals.TIMG0).timer0;

    // We need second timer for Embassy to work
    let embassy_timer = TimerGroup::new(peripherals.TIMG1).timer0;
    esp_hal_embassy::init(embassy_timer);

    let led = led_init(peripherals.GPIO2).await;

    rtc::init(peripherals.LPWR).await;

    update_status("App core starting").await.unwrap();

    let _appcore_guard = start_appcore(peripherals.CPU_CTRL, {
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
        }
    })
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

    let ntp = NtpClient::new(&stack);
    ntp.sync().await.ok();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        Timer::after(Duration::from_millis(1_000)).await;
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        let url = "www.mobile-j.de";

        let remote_address = stack.dns_query(url, DnsQueryType::A).await.unwrap();
        let remote_endpoint = (remote_address[0], 80);
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
        Timer::after(Duration::from_millis(20000)).await;
    }
}
