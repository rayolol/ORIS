use defmt::debug;
use embedded_hal::digital::OutputPin;
use crate::hal::{Backend, State, Condition, DeviceState};
use heapless::String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LedData {
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedState {
    pub on: bool,
}

impl Default for LedState {
    fn default() -> Self {
        Self { on: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedError {
    PinError,
}

impl Condition for LedError {
    fn classify(&self, _ctx: &DeviceState<impl State>) -> crate::hal::Fault {
        let mut msg = String::<64>::new();
        let _ = msg.push_str("LED error");
        crate::hal::Fault {
            severity: crate::hal::Severity::Fault,
            message: msg,
        }
    }
}

use crate::hal::Lane;

pub struct LedBackend<PIN: OutputPin, L: Lane<LedData> + 'static> {
    pin: PIN,
    state: LedState,
    bus: &'static L,
}

impl<PIN: OutputPin, L: Lane<LedData> + 'static> LedBackend<PIN, L> {
    pub fn new(pin: PIN, bus: &'static L) -> Self {
        Self {
            pin,
            state: LedState::default(),
            bus,
        }
    }

    pub fn set_on(&mut self) -> Result<(), LedError> {
        debug!("led: turning on");
        self.pin.set_low().map_err(|_| LedError::PinError)?;
        self.state.on = true;
        Ok(())
    }

    pub fn set_off(&mut self) -> Result<(), LedError> {
        debug!("led: turning off");
        self.pin.set_high().map_err(|_| LedError::PinError)?;
        self.state.on = false;
        Ok(())
    }

    pub fn toggle(&mut self) -> Result<(), LedError> {
        if self.state.on {
            self.set_off()
        } else {
            self.set_on()
        }
    }
}

impl<PIN: OutputPin, L: Lane<LedData> + 'static> Backend for LedBackend<PIN, L> {
    type Output = LedState;
    type Condition = LedError;
    type Config = ();
    type Error = LedError;

    async fn init(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
        debug!("led backend: initializing");
        self.set_off()
    }

    async fn tick(&mut self) -> Self::Output {
        if let Some(data) = self.bus.read() {
            debug!("led backend: read enabled={}", data.enabled);
            if data.enabled && !self.state.on {
                debug!("led backend: turning ON");
                let _ = self.set_on();
            } else if !data.enabled && self.state.on {
                debug!("led backend: turning OFF");
                let _ = self.set_off();
            }
        } else {
            debug!("led backend: no data from bus");
        }
        self.state
    }

    async fn config(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
        Ok(())
    }
}
