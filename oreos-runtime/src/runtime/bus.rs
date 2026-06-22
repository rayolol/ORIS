use heapless::Vec;

use crate::hal::{EstopFlag, GenericBus};
use crate::transport::bus::{FastLane, SlowLane};
use crate::drivers::motor::StepperData;
use crate::drivers::driver::DriverData;
use crate::drivers::tmc2160::structs::DriverStatus as Tmc2160Data;


pub struct ArmJointBus {

    pub motor:      FastLane<StepperData>,
    pub driver_209: SlowLane<DriverData>,
    pub estop:      EstopFlag,
}

impl ArmJointBus {
    pub const fn new() -> Self {
        Self {
            motor:      FastLane::new(StepperData { position_steps: 0, vel_steps_per_s: 0 }),
            driver_209: SlowLane::new(),
            estop:      EstopFlag::new(),
        }
    }


}

impl GenericBus<super::kernel::KernelState> for ArmJointBus {
    fn estop(&self) -> &EstopFlag {
        &self.estop
    }

    fn update(&self, _state: &mut super::kernel::KernelState) {
        todo!()
    }

    fn write(&self, _state: &super::kernel::KernelState) {
        todo!()
    }
    
}

