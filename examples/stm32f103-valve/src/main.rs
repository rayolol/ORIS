#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use core::sync::atomic::{AtomicBool, Ordering};

use oreos::prelude::*;
use oreos_stm32f103_valve_example::{ValveCommand, ValveDevice, make_valve_with_stm32_pins};

static TIME_READY: AtomicBool = AtomicBool::new(false);

fn timestamp_micros() -> u64 {
    if TIME_READY.load(Ordering::Relaxed) {
        embassy_time::Instant::now().as_micros()
    } else {
        0
    }
}

defmt::timestamp!("{=u64}", timestamp_micros());

#[defmt::panic_handler]
fn defmt_panic() -> ! {
    panic_probe::hard_fault()
}

#[devices]
struct RuntimeDevices {
    #[device]
    valve: ValveDevice,
}

#[app(hal_crate = embassy_stm32)]
mod app {
    #[init(config = embassy_stm32::Config::default())]
    async fn init(p: embassy_stm32::Peripherals, _: Spawner) -> Devices {
        TIME_READY.store(true, Ordering::Relaxed);
        Devices {
            valve: make_valve_with_stm32_pins(p),
        }
    }

    #[loop_(rate = 10, priority = 0)]
    async fn control(ctx: Context) {
        let seconds = embassy_time::Instant::now().as_secs();
        let command = if (seconds / 2) % 2 == 0 {
            ValveCommand::Open
        } else {
            ValveCommand::Close
        };

        ctx.valve.execute(command);
        ctx.valve
            .tick(oreos::fugit::Duration::<u32, 1, 1000>::from_millis(10));
    }
}
