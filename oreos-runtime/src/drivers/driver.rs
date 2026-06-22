use crate::hal::{Backend, Condition, DeviceState, Severity, Fault, State};
use defmt::{info, trace, error};
use heapless::String;
use crate::drivers::tmc2209::tmc2209::{Tmc2209};
use crate::drivers::tmc2209::structs::{Tmc2209Config, Tmc2209Error};
use crate::drivers::tmc2160::tmc2160::Tmc2160;
use crate::drivers::tmc2160::structs::{Tmc2160Config, Tmc2160Error};

use crate::transport::traits::TransportHandler;

#[derive(Copy, Clone, Default)]
pub struct DriverData {
    pub stalled: bool,
    pub otp: bool,
    pub otpw: bool,
    pub sg_result: u16,
}

#[derive(Default)]
pub struct RawDriverStatus {
    pub over_temp_shutdown: bool,
    pub over_temp_prewarning: bool,
}

pub struct DriverCondition {
    pub stalled: bool,
    pub overtemp: bool,
    pub overtemp_warning: bool,
}

impl Condition for DriverCondition {
    fn classify(&self, _ctx: &DeviceState<impl State>) -> Fault {
        if self.overtemp {
            return Fault {
                severity: Severity::Fault,
                message: String::try_from("driver overtemperature").unwrap(),
            };
        }
        if self.stalled {
            return Fault {
                severity: Severity::Warning,
                message: String::try_from("stall detected").unwrap(),
            };
        }
        Fault { severity: Severity::Healthy, message: String::new() }
    }
}

// ── StepperDriver trait ──────────────────────────────────────────────────────

#[allow(async_fn_in_trait)]
pub trait StepperDriver {
    type Config;
    type Error;

    async fn init(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    async fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    async fn status(&mut self) -> Result<RawDriverStatus, Self::Error>;
    async fn stall_result(&mut self) -> Result<u16, Self::Error>;
}

// ── StepperDriver impls ──────────────────────────────────────────────────────

#[allow(async_fn_in_trait)]
impl<T: TransportHandler<8>> StepperDriver for Tmc2209<T> {
    type Config = Tmc2209Config;
    type Error = Tmc2209Error<T::Error>;

    async fn init(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.init(config).await
    }

    async fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.reconfigure(config).await
    }

    async fn status(&mut self) -> Result<RawDriverStatus, Self::Error> {
        let s = self.status().await?;
        Ok(RawDriverStatus {
            over_temp_shutdown: s.over_temp_shutdown,
            over_temp_prewarning: s.over_temp_prewarning,
        })
    }

    async fn stall_result(&mut self) -> Result<u16, Self::Error> {
        self.stall_result().await
    }
}


#[allow(async_fn_in_trait)]
impl<T: TransportHandler<5>> StepperDriver for Tmc2160<T> {
    type Config = Tmc2160Config;
    type Error = Tmc2160Error<T::Error>;

    async fn init(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.init(config).await
    }

    async fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.reconfigure(config).await
    }

    async fn status(&mut self) -> Result<RawDriverStatus, Self::Error> {
        let s = self.status().await?;
        Ok(RawDriverStatus {
            over_temp_shutdown: s.over_temp_shutdown,
            over_temp_prewarning: s.over_temp_prewarning,
        })
    }

    async fn stall_result(&mut self) -> Result<u16, Self::Error> {
        self.stall_result().await
    }
}

// ── Generic backend ──────────────────────────────────────────────────────────

pub struct DriverBackend<D: StepperDriver> {
    driver: D,
}

impl<D: StepperDriver> DriverBackend<D> {
    pub const fn new(driver: D) -> Self {
        Self { driver }
    }
}

#[allow(async_fn_in_trait)]
impl<D: StepperDriver> Backend for DriverBackend<D> {
    type Output = DriverData;
    type Config = D::Config;
    type Error = D::Error;
    type Condition = DriverCondition;

    async fn init(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        let r = self.driver.init(config).await;
        if r.is_err() {
            error!("error occurred in driver init");
        }
        info!("stepper driver successfully initiated");
        Ok(())
    }

    async fn tick(&mut self) -> Self::Output {
        let status = self.driver.status().await.unwrap_or_default();
        let sg = self.driver.stall_result().await.unwrap_or(0);
        // trace!("driver tick got this status: {} with stallguard value: {}", status, sg);

        DriverData {
            stalled: sg == 0,
            otp: status.over_temp_shutdown,
            otpw: status.over_temp_prewarning,
            sg_result: sg,
        }
    }

    async fn config(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        let r = self.driver.reconfigure(config).await;
        if r.is_err() {
            error!("error occurred in driver config");
        }
        info!("config initiated");
        Ok(())
    }
}
