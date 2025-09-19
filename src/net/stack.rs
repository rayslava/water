use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_wifi::wifi::WifiDevice;
use static_cell::StaticCell;

use crate::error::SysError;

pub async fn init_net(
    driver: WifiDevice<'static>,
    seed: u64,
    spawner: &Spawner,
) -> Result<Stack<'static>, SysError> {
    let resources = {
        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        RESOURCES.init(StackResources::<3>::new())
    };
    let config = embassy_net::Config::dhcpv4(Default::default());

    let (stack, runner) = embassy_net::new(driver, config, resources, seed);
    spawner.spawn(net_task(runner))?;
    Ok(stack)
}

// We have to run the task in background to make the stack work
#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

const NET_REFRESH_TIME: Duration = Duration::from_millis(500);

pub async fn wait_for_link(stack: Stack<'static>) {
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(NET_REFRESH_TIME).await;
    }
}

pub async fn wait_for_ip(stack: Stack<'static>) -> Ipv4Cidr {
    loop {
        if let Some(config) = stack.config_v4() {
            return config.address;
        }
        Timer::after(NET_REFRESH_TIME).await;
    }
}
