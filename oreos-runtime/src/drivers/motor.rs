use crate::hal::{Backend, Condition, DeviceState, Fault, Severity, State};
use super::stepper::stepper::{Stepper, StepperError, StepperConfig};

#[derive(Clone, Copy, Default)]
pub struct StepperData {
    pub position_steps: i32,
    pub vel_steps_per_s: i32,
}

#[derive(Clone, Copy)]
pub struct MotorCondition {
    pub stalled: bool,
    pub overtemp: bool,
    pub overtemp_warning: bool,
}

impl Condition for MotorCondition {
    // DEMO: simple priority — stall > overtemp shutdown > overtemp warning > healthy.
    fn classify(&self, _ctx: &DeviceState<impl State>) -> Fault {
        let (severity, message) = if self.stalled {
            (Severity::Fault, "motor stalled")
        } else if self.overtemp {
            (Severity::Fault, "motor overtemp")
        } else if self.overtemp_warning {
            (Severity::Warning, "motor overtemp warning")
        } else {
            (Severity::Healthy, "ok")
        };
        let mut msg = heapless::String::new();
        let _ = msg.push_str(message);
        Fault { severity, message: msg }
    }
}

pub struct StepDirBackend<STEP, DIR, EN> {
    stepper: Stepper<STEP, DIR, EN>,
}

impl<STEP, DIR, EN> Backend for StepDirBackend<STEP, DIR, EN>
where
    STEP: embedded_hal::digital::OutputPin,
    DIR: embedded_hal::digital::OutputPin,
    EN: embedded_hal::digital::OutputPin,
{
    type Output = StepperData;
    type Condition = MotorCondition;
    type Config = StepperConfig;
    type Error = StepperError;

    async fn config(&mut self, config: StepperConfig) -> Result<(), StepperError> {
        self.stepper.hardware_config = config;
        self.stepper.control_hz = config.steps_per_rev as u32 * config.microsteps as u32;
        Ok(())
    }

    async fn init(&mut self, config: StepperConfig) -> Result<(), StepperError> {
        self.stepper.hardware_config = config;
        self.stepper.enable()
    }

    async fn tick(&mut self) -> Self::Output {
        let _ = self.stepper.tick();
        StepperData {
            position_steps: self.stepper.step_counter,
            vel_steps_per_s: if self.stepper.dir_positive {
                self.stepper.current_vel_steps_s as i32
            } else {
                -(self.stepper.current_vel_steps_s as i32)
            },
        }
    }
}
