use crate::hal::{Device, DeviceConfig, DeviceState, Kernel, Mode, Config, MaybeDevcie};
use crate::motion::{MotionPlanners, Trapezoidal};
use super::command::{ActuatorCommand, ApplyCommand};
use crate::runtime::kernel::{KernelConfig, KernelState};


pub trait ActuatorConfig: Config {}

pub struct ActuatorConf {

}

impl Config for ActuatorConf {}
impl ActuatorConfig for ActuatorConf {}


pub struct ActuatorNode<K: Kernel<State = KernelState, Config = KernelConfig>> {
    kernel: K,
    state:  DeviceState<K::State>,
    motion: MotionPlanners,
    config: DeviceConfig<K::Config>,
}

// DEMO: constructor — wires a kernel and its config into a ready-to-tick node.
impl<K: Kernel<State = KernelState, Config = KernelConfig>> ActuatorNode<K> {
    pub fn new(kernel: K, config: DeviceConfig<KernelConfig>) -> Self {
        Self {
            kernel,
            state: DeviceState {
                enabled: false,
                mode:    Mode::Idle,
                fault:   None,
                custom:  KernelState::default(),
            },
            motion: MotionPlanners::Trapezoidal(Trapezoidal::new()),
            config,
        }
    }
}

impl<K: Kernel<State = KernelState, Config = KernelConfig>> Device for ActuatorNode<K> {
    type Kernel = K;
    type Command = ();

    fn kernel(&mut self) -> &mut Self::Kernel {
        &mut self.kernel
    }

    fn tick(&mut self, dt: fugit::Duration<u32, 1, 1000>) {
        self.kernel.tick();
        self.state.custom = self.kernel.feedback();

        let setpoint = self.motion.update(dt.as_micros(), &self.state.custom);

        // DEMO: push the velocity setpoint back into KernelState so the kernel's
        // writer fn can forward it to the stepper via the bus.
        self.state.custom.target_vel_rad_s = setpoint.vel_rad_s;

        // DEMO: update mode — Active while moving, Idle when the planner signals done.
        if setpoint.done {
            self.state.mode = Mode::Idle;
        } else {
            self.state.mode = Mode::Active;
        }
    }

    fn execute(&mut self, _cmd: Self::Command) {
    }
}
