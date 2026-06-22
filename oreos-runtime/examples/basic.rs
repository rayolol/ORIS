#![no_std]
#![no_main]

use robot_arm_firmware_v2::prelude::*;

#[derive(Default, State, Clone, Copy)]
struct MyState {
    pub speed: u32,
    pub name: &'static str,
}

#[derive(Default, Config, Clone, Copy)]
struct MyConfig {
    black: u32,
}

#[derive(Copy, Clone)]
struct Data {
    pub field: u32,
}

#[derive(Copy, Clone)]
struct NameRegistry {
    name: &'static str,
}

#[derive(GenericBus)]
struct MyBus {
    #[state]
    state: MyState,
    estop: EstopFlag,
    #[updates(MyState::speed => Data::field)]
    lane1: FastLane<Data>,
    #[updates(MyState::name => NameRegistry::name)]
    lane2: SlowLane<NameRegistry>,
}

#[derive(Kernel)]
struct MyKernel {
    #[state]
    state: MyState,
    #[config]
    config: MyConfig,
    #[bus]
    bus: MyBus,
}

#[create(Device)]
struct MyDevice {
    #[kernel]
    kernel: MyKernel,
    #[state]
    state: DeviceState<MyState>,
    #[config]
    config: DeviceConfig<MyConfig>,
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(export_name = "main")]
pub extern "C" fn main() -> ! {
    loop {}
}
