use defmt::trace;
use embedded_hal::digital::{OutputPin, PinState};
use crate::{config::HardwareConfig, drivers::motor::StepperData, hal::{Lane, Backend, Condition, DeviceState, State, Fault, Severity}, transport::bus::FastLane};
use heapless::String;


#[derive(Clone, Copy)]
pub struct StepperConfig {
    pub steps_per_rev: u32,
    pub microsteps: u16,
    pub invert_dir: bool,
    pub enable_active_low: bool,
    pub pulse_width_ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepperError {
    Stall,
    NotAvailable,
    InvalidVelocity,
    HalError
}

impl Condition for StepperError {
    fn classify(&self, _ctx: &DeviceState<impl State>) -> Fault {
        let msg_str = match self {
            StepperError::Stall => "Motor stall detected",
            StepperError::NotAvailable => "Motor not available",
            StepperError::InvalidVelocity => "Invalid velocity command",
            StepperError::HalError => "Hardware abstraction layer error",
        };
        let mut message = String::<64>::new();
        let _ = message.push_str(msg_str);
        Fault {
            severity: Severity::Fault,
            message,
        }
    }
}

pub struct Stepper<STEP, DIR, EN> {
    step: STEP,
    dir: DIR,
    en: EN,
    pub hardware_config: StepperConfig,
    pub enabled: bool,
    pub dir_positive: bool,
    pub step_counter: i32,
    pub current_vel_steps_s: u32,
    step_state_high: bool,
    ticks_until_edge: u32,
    step_period_ticks: u32,
    pulse_width_ticks: u32,
    pub control_hz: u32,
}


impl<STEP, DIR, EN> Stepper<STEP, DIR, EN>
where
    STEP: OutputPin,
    DIR: OutputPin,
    EN: OutputPin,
{
    pub fn new(
        step: STEP,
        dir: DIR,
        en: EN,
        control_hz: u32,
        hardware_config: StepperConfig,
    ) -> Self {
        Self {
            step,
            dir,
            en,
            hardware_config,
            enabled: false,
            dir_positive: true,
            step_counter: 0,
            current_vel_steps_s: 0,
            step_state_high: false,
            ticks_until_edge: 0,
            step_period_ticks: 0,
            pulse_width_ticks: u32::from(hardware_config.pulse_width_ticks).max(1),
            control_hz,
        }
    }

    fn set_enable_state(&mut self, enabled: bool) -> Result<(), StepperError> {
        let pin_is_high = if self.hardware_config.enable_active_low {
            !enabled
        } else {
            enabled
        };
        self.en
            .set_state(PinState::from(pin_is_high))
            .map_err(|_| StepperError::HalError)
    }

    pub fn set_velocity_steps_s(&mut self, vel_steps_s: i32) {
        defmt::debug!("stepper: set velocity vel={}", vel_steps_s);
        if vel_steps_s > 0 {
            let _ = self.set_direction(true);
            self.current_vel_steps_s = vel_steps_s as u32;
        } else if vel_steps_s < 0 {
            let _ = self.set_direction(false);
            self.current_vel_steps_s = vel_steps_s.unsigned_abs();
        } else {
            self.current_vel_steps_s = 0;
            // Don't touch ticks_until_edge — let the guard in tick() handle the clean stop.
        }
        self.step_period_ticks = self.control_hz / (2 * self.current_vel_steps_s).max(1);
    }

    pub fn set_direction(&mut self, positive: bool) -> Result<(), StepperError> {
        defmt::debug!("stepper: set direction dir={}", positive);
        self.dir_positive = positive;
        let pin_is_high = if self.hardware_config.invert_dir {
            !positive
        } else {
            positive
        };
        self.dir
            .set_state(PinState::from(pin_is_high))
            .map_err(|_| StepperError::HalError)
    }

    pub fn tick(&mut self) ->  Result<(), StepperError> {
        if !self.enabled || self.current_vel_steps_s == 0 {
            return Ok(());
        }

        trace!("stepper tick: vel={}, pos={}", self.current_vel_steps_s, self.step_counter);

        if self.ticks_until_edge > 0 {
            self.ticks_until_edge -= 1;
            return Ok(());
        }

        self.step_state_high = !self.step_state_high;
        self.step
            .set_state(PinState::from(self.step_state_high))
            .map_err(|_| {defmt::error!("step pin error"); StepperError::HalError})?;

        if self.step_state_high {
            self.ticks_until_edge = self.pulse_width_ticks;
        } else {
            if self.dir_positive {
                self.step_counter = self.step_counter.wrapping_add(1);
            } else {
                self.step_counter = self.step_counter.wrapping_sub(1);
            }

            let low_ticks = self
                .step_period_ticks
                .saturating_sub(self.pulse_width_ticks)
                .max(1);
            self.ticks_until_edge = low_ticks;
        }

        Ok(())
    }

    pub fn enable(&mut self) -> Result<(), StepperError> {
        defmt::info!("stepper: enabling motor");
        self.enabled = true;
        self.set_enable_state(true)
    }

    pub fn disable(&mut self) -> Result<(), StepperError> {
        defmt::info!("stepper: disabling motor");
        self.enabled = false;
        self.current_vel_steps_s = 0;
        self.step_state_high = false;
        self.step.set_low().map_err(|_| StepperError::HalError)?;
        self.set_enable_state(false)
    }

    // fn status(&self) -> MotorStatus {
    //     MotorStatus {
    //         enabled: self.enabled,
    //         pos_steps: self.step_counter,
    //         vel_steps_s: if self.current_vel_steps_s == 0 {
    //             0
    //         } else if self.dir_positive {
    //             self.current_vel_steps_s as i32
    //         } else {
    //             -(self.current_vel_steps_s as i32)
    //         },
    //         busy: self.enabled && self.current_vel_steps_s > 0,
    //         faulted: false,
    //     }
    // }

}

// ─── StepperBackend ───────────────────────────────────────────────────────────


// make sync later
pub struct StepperBackend<STEP, DIR, EN, L: Lane<StepperData> + 'static> {
    pub(crate) stepper: Stepper<STEP, DIR, EN>,
    bus: &'static L
}

impl<STEP, DIR, EN, L: Lane<StepperData> + 'static> StepperBackend<STEP, DIR, EN, L>
where
    STEP: OutputPin,
    DIR: OutputPin,
    EN: OutputPin,
{
    pub fn new(step: STEP, dir: DIR, en: EN, control_hz: u32, bus: &'static L) -> Self {
        // change later
        let cfg = StepperConfig {
            steps_per_rev: 200,
            microsteps: 16,
            invert_dir: true,
            enable_active_low:true ,
            pulse_width_ticks: 100 as u32,
        };
        Self {
            stepper: Stepper::new(step, dir, en, control_hz, cfg),
            bus
        }
    }

    pub async fn tick(&mut self) {
        if let Some(data) = self.bus.read() {
            defmt::debug!("stepper backend: read from bus vel={}", data.vel_steps_per_s);
            self.stepper.set_velocity_steps_s(data.vel_steps_per_s);
        }
        let feedback = self.feedback();
        defmt::trace!("stepper backend: write feedback pos={}, vel={}", feedback.position_steps, feedback.vel_steps_per_s);
        self.bus.write(feedback);
        let _ = self.stepper.tick();
    }

    pub fn enable(&mut self) -> Result<(), StepperError> {
        defmt::info!("stepper backend: enabling");
        self.stepper.enable()
    }

    pub fn feedback(&self) -> StepperData {
        StepperData {
            position_steps: self.stepper.step_counter,
            vel_steps_per_s: if self.stepper.dir_positive {
                self.stepper.current_vel_steps_s as i32
            } else {
                -(self.stepper.current_vel_steps_s as i32)
            },
        }
    }

    pub fn disable(&mut self) {
        let _ = self.stepper.disable();
    }
}

impl<STEP, DIR, EN, L: Lane<StepperData> + 'static> Backend for StepperBackend<STEP, DIR, EN, L>
where
    STEP: OutputPin,
    DIR: OutputPin,
    EN: OutputPin,
{
    type Output = StepperData;
    type Condition = StepperError;
    type Config = StepperConfig;
    type Error = StepperError;

    async fn init(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
        defmt::info!("stepper backend: initializing");
        self.enable()
    }

    async fn tick(&mut self) -> Self::Output {
        StepperBackend::tick(self).await;
        self.feedback()
    }

    async fn config(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.stepper.hardware_config = config;
        Ok(())
    }
}
