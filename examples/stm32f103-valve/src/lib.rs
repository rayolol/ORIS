#![no_std]

use embassy_stm32::Peri;
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::gpio::{AfioRemap, Input, Level, Output, OutputType, Pull, Speed};
use embassy_stm32::peripherals::{ADC1, PA2, TIM2};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::Ch2;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm, SimplePwmChannel};
use oreos::prelude::*;

#[derive(Clone, Copy, Default, State)]
pub struct ValveState {
    pub requested_open: bool,
    pub actual_open: bool,
    pub analog_raw: u16,
}

#[derive(Clone, Copy, Default, Config)]
pub struct ValveConfig {}

#[derive(Command)]
pub enum ValveCommand {
    Open,
    Close,
}

#[derive(Clone, Copy, Default)]
pub struct ValveData {
    pub requested_open: bool,
    pub actual_open: bool,
    pub analog_raw: u16,
}

#[derive(GenericBus)]
pub struct ValveBus {
    #[state]
    state: ValveState,
    estop: EstopFlag,
    #[route(
        ValveState::requested_open => ValveData::requested_open,
        ValveState::actual_open <= ValveData::actual_open,
        ValveState::analog_raw <= ValveData::analog_raw
    )]
    valve: FastLane<ValveData>,
}

#[derive(Kernel)]
pub struct ValveKernel {
    #[state]
    state: ValveState,
    #[config]
    config: ValveConfig,
    #[bus]
    bus: &'static ValveBus,
}

mod access {
    use super::{ADC1, Adc, Input, IoAccess, Output, PA2, Peri, SimplePwmChannel, TIM2};

    #[derive(IoAccess)]
    pub struct ValveAccess {
        #[io(kind = "digital")]
        output: Output<'static>,
        #[io(kind = "digital")]
        feedback: Input<'static>,
        #[io(kind = "analog")]
        pub(crate) adc: Adc<'static, ADC1>,
        #[io(kind = "analog")]
        pub(crate) analog_pin: Peri<'static, PA2>,
        #[io(kind = "pwm")]
        motor: SimplePwmChannel<'static, TIM2>,
    }
}

use access::ValveAccess;

pub struct ValveCondition;
impl Condition for ValveCondition {
    fn classify(&self, _ctx: &DeviceState<impl StateTrait>) -> Fault {
        Fault {
            severity: Severity::Healthy,
            message: Default::default(),
        }
    }
}

pub struct ValveBackend {
    lane: &'static FastLane<ValveData>,
    access: ValveAccess,
}

impl ValveBackend {
    pub fn new(lane: &'static FastLane<ValveData>, access: ValveAccess) -> Self {
        Self { lane, access }
    }
}

impl Backend for ValveBackend {
    type Output = ValveData;
    type Condition = ValveCondition;
    type Config = ();
    type Error = ();

    async fn init(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
        self.access.output_mut().set_low();
        self.access.motor_mut().set_duty_cycle_percent(0);
        self.access.motor_mut().enable();
        Ok(())
    }

    async fn tick(&mut self) -> Self::Output {
        let mut data = self.lane.read().unwrap_or_default();
        if data.requested_open {
            self.access.output_mut().set_high();
            self.access.motor_mut().set_duty_cycle_percent(30);
        } else {
            self.access.output_mut().set_low();
            self.access.motor_mut().set_duty_cycle_percent(0);
        }
        data.actual_open = self.access.feedback_mut().is_high();
        data.analog_raw = self
            .access
            .adc
            .read(&mut self.access.analog_pin, SampleTime::CYCLES28_5)
            .await;
        self.lane.write(data);
        data
    }

    async fn config(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct ValvePolicy;
impl Middleware<DeviceState<ValveState>, DeviceConfig<ValveConfig>, ValveCommand> for ValvePolicy {
    fn process(
        &mut self,
        _state: &mut DeviceState<ValveState>,
        _config: &DeviceConfig<ValveConfig>,
    ) {
    }

    fn command(
        &mut self,
        command: ValveCommand,
        state: &mut DeviceState<ValveState>,
        _config: &DeviceConfig<ValveConfig>,
    ) {
        state.custom.requested_open = matches!(command, ValveCommand::Open);
    }
}

#[create(Device)]
pub struct ValveDevice {
    #[kernel]
    kernel: ValveKernel,
    #[state]
    state: DeviceState<ValveState>,
    #[config]
    config: DeviceConfig<ValveConfig>,
    #[backend(tick_rate: "10")]
    backend: ValveBackend,
    #[middleware]
    middleware: ValvePolicy,
}

pub fn make_valve_with_stm32_pins(p: embassy_stm32::Peripherals) -> ValveDevice {
    let output = Output::new(p.PB0, Level::Low, Speed::Low);
    let feedback = Input::new(p.PB1, Pull::Down);
    let adc = Adc::new(p.ADC1);
    let pwm_pin = PwmPin::<TIM2, Ch2, AfioRemap<0>>::new(p.PA1, OutputType::PushPull);
    let pwm = SimplePwm::new(
        p.TIM2,
        None,
        Some(pwm_pin),
        None,
        None,
        Hertz(1_000),
        CountingMode::EdgeAlignedUp,
    );
    let motor = pwm.split().ch2;
    let access = ValveAccess::new(output, feedback, adc, p.PA2, motor);

    let bus = ValveBus::new(
        FastLane::new(ValveData::default()),
        ValveState::default(),
        EstopFlag::new(),
    );
    let backend = ValveBackend::new(bus.valve(), access);
    let kernel = ValveKernel::new(ValveState::default(), ValveConfig::default(), bus);
    ValveDevice::new(
        backend,
        kernel,
        DeviceState::default(),
        DeviceConfig::default(),
        ValvePolicy,
    )
}
