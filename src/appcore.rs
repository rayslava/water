use core::ptr::addr_of_mut;

use embassy_executor::Spawner;
use esp_hal::interrupt::software::SoftwareInterrupt;
use esp_hal::peripherals::CPU_CTRL;
use esp_hal::system::Stack;
use esp_rtos::embassy::Executor;
use static_cell::StaticCell;

use crate::error::SysError;

static mut APP_CORE_STACK: Stack<8192> = Stack::new();

pub fn start_appcore<F>(
    cpu_peripheral: CPU_CTRL<'static>,
    int0: SoftwareInterrupt<'static, 0>,
    int1: SoftwareInterrupt<'static, 1>,
    task_spawner: F,
) -> Result<(), SysError>
where
    F: FnOnce(Spawner) + Send + 'static,
{
    esp_rtos::start_second_core(
        cpu_peripheral,
        int0,
        int1,
        unsafe { &mut *addr_of_mut!(APP_CORE_STACK) },
        move || {
            // Create executor for the second core
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());

            // Run the executor with the provided task spawner
            executor.run(|spawner| {
                task_spawner(spawner);
            });
        },
    );
    // Note: start_second_core doesn't return a result in esp-rtos, it panics on failure

    Ok(())
}
