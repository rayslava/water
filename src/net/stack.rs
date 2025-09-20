use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_wifi::wifi::WifiDevice;
use heapless::String;
use static_cell::StaticCell;

use crate::{
    display::{STATUS_LEN, update_status},
    error::SysError,
    io::led::{HEARTBEAT_NET_AWAIT, set_heartbeat},
};

// Available number of sockets for the network stack
const SOCKETS: usize = 10;

pub async fn init_net(
    driver: WifiDevice<'static>,
    seed: u64,
    spawner: &Spawner,
) -> Result<&'static mut Stack<'static>, SysError> {
    let resources = {
        static RESOURCES: StaticCell<StackResources<SOCKETS>> = StaticCell::new();
        RESOURCES.init(StackResources::<SOCKETS>::new())
    };
    let config = embassy_net::Config::dhcpv4(Default::default());

    let (stack, runner) = embassy_net::new(driver, config, resources, seed);
    spawner.spawn(net_task(runner))?;
    let stack = {
        static RESOURCES: StaticCell<Stack<'static>> = StaticCell::new();
        RESOURCES.init(stack)
    };
    Ok(stack)
}

// We have to run the task in background to make the stack work
#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

const NET_REFRESH_TIME: Duration = Duration::from_millis(500);

pub async fn wait_for_link(stack: &Stack<'static>) {
    set_heartbeat(HEARTBEAT_NET_AWAIT);
    update_status("Waiting for net").await.ok();
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(NET_REFRESH_TIME).await;
    }
}

pub async fn wait_for_ip(stack: &Stack<'static>) -> Ipv4Cidr {
    set_heartbeat(HEARTBEAT_NET_AWAIT);
    update_status("Waiting for IP").await.ok();
    loop {
        if let Some(config) = stack.config_v4() {
            let mut ip_string: String<STATUS_LEN> = String::new();
            write!(ip_string, "IP: {}", config.address.address()).unwrap();
            update_status(&ip_string).await.unwrap();

            return config.address;
        }
        Timer::after(NET_REFRESH_TIME).await;
    }
}
