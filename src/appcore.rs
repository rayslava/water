use core::ptr::addr_of_mut;

use embassy_executor::Spawner;
use esp_hal::peripherals::CPU_CTRL;
use esp_hal::system::{AppCoreGuard, CpuControl, Stack};
use esp_hal_embassy::Executor;
use static_cell::StaticCell;

use crate::error::SysError;

static mut APP_CORE_STACK: Stack<8192> = Stack::new();

pub fn start_appcore<F>(
    cpu_peripheral: CPU_CTRL<'static>,
    task_spawner: F,
) -> Result<AppCoreGuard<'static>, SysError>
where
    F: FnOnce(Spawner) + Send + 'static,
{
    let mut cpu_control = CpuControl::new(cpu_peripheral);

    Ok(
        cpu_control.start_app_core(unsafe { &mut *addr_of_mut!(APP_CORE_STACK) }, move || {
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());

            executor.run(|spawner: Spawner| {
                task_spawner(spawner);
            });
        })?,
    )
}
