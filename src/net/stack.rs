use embassy_net::{Runner, Stack, StackResources};
use esp_wifi::wifi::WifiDevice;
use static_cell::StaticCell;

pub async fn init_net(
    driver: WifiDevice<'static>,
    seed: u64,
) -> (Stack<'static>, Runner<'static, WifiDevice<'static>>) {
    let resources = {
        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        RESOURCES.init(StackResources::<3>::new())
    };
    let config = embassy_net::Config::dhcpv4(Default::default());

    embassy_net::new(driver, config, resources, seed)
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
