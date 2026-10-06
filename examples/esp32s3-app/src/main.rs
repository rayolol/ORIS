#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use oreos::prelude::*;

esp_bootloader_esp_idf::esp_app_desc!();

defmt::timestamp!("");

#[defmt::panic_handler]
fn defmt_panic() -> ! {
    panic!("defmt assertion failed")
}

pub struct InterfaceState {
    updates: u32,
    uptime_ms: u64,
}

#[devices]
struct AppDevices {
    interface: InterfaceState,
}

#[app(hal_crate = esp_hal, entry = esp_rtos::main)]
mod app {
    #[init(config = esp_hal::Config::default())]
    async fn init(p: esp_hal::peripherals::Peripherals, _: Spawner) -> Devices {
        // ESP-specific setup stays in the application's init function.
        let timers = TimerGroup::new(p.TIMG0);
        esp_rtos::start(timers.timer0, p.FROM_CPU_INTR0);

        // Initialize this board's display/touch drivers here once its exact
        // model and pinout are known, and return them in Devices as well.
        esp_println::println!("OREOS initialized on ESP32-S3");
        Devices {
            interface: InterfaceState {
                updates: 0,
                uptime_ms: 0,
            },
        }
    }

    // `rate` is the delay between iterations, in milliseconds.
    #[loop_(rate = 1000)]
    async fn update_interface(ctx: Context) {
        ctx.interface.updates += 1;
        ctx.interface.uptime_ms = embassy_time::Instant::now().as_millis();
        esp_println::println!(
            "OREOS update {}: uptime {} ms",
            ctx.interface.updates,
            ctx.interface.uptime_ms,
        );
    }
}
