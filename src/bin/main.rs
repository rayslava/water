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

use core::ptr::addr_of_mut;

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::system::{CpuControl, Stack};
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_hal_embassy::Executor;
use esp_println::println;
use static_cell::StaticCell;
use water::display::update_status;
use water::io::gpio::led_init;
use water::io::led::heartbeat;
use water::io::wifi::{maintain_connection, wifi_hw_init};
use water::net::stack::{init_net, net_task};
use water::{display, io::i2c::display_i2c_init};
esp_bootloader_esp_idf::esp_app_desc!();

static mut APP_CORE_STACK: Stack<8192> = Stack::new();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let mut display_i2c =
        display_i2c_init(peripherals.I2C0, peripherals.GPIO21, peripherals.GPIO22)
            .await
            .unwrap();
    println!("Init display I2C");
    let mut display = display::init(&mut display_i2c).await.unwrap();
    update_status("WiFi init", &mut display).await.unwrap();

    let wifi = wifi_hw_init(timg0.timer0, rng, peripherals.WIFI)
        .await
        .unwrap();

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    update_status("LED init", &mut display).await.unwrap();
    let led = led_init(peripherals.GPIO2).await;

    update_status("CPU 2 init", &mut display).await.unwrap();
    let mut cpu_control = CpuControl::new(peripherals.CPU_CTRL);

    let _guard = cpu_control
        .start_app_core(unsafe { &mut *addr_of_mut!(APP_CORE_STACK) }, move || {
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());
            executor.run(|spawner| {
                spawner.spawn(heartbeat(led.led, led.control_signal)).ok();
            });
        })
        .unwrap();

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = init_net(wifi.interface, seed).await;

    spawner.spawn(maintain_connection(wifi.controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        led.control_signal.signal(true);
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
        led.control_signal.signal(false);
    }
}
