use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources};
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

// We have to run the function in background to make the stack work
#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
