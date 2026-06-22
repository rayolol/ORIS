// Generic hardware abstraction layer.
//
// Everything in here is reusable and has no knowledge of this specific robot
// arm.  Application-specific types live in `api/`, `config/`, and `runtime/`.

use core::sync::atomic::{AtomicBool, Ordering};

// ── Bus Primitives ──────────────────────────────────────────────────────────

pub trait Lane<T> {
    fn write(&self, data: T);
    fn read(&self) -> Option<T>;
}

impl<T: Copy, L: Lane<T> + ?Sized> Lane<T> for &L {
    fn write(&self, data: T) {
        L::write(self, data);
    }

    fn read(&self) -> Option<T> {
        L::read(self)
    }
}
pub struct EstopFlag(AtomicBool);

impl EstopFlag {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    pub fn set(&self) {
        self.0.store(true, Ordering::Release)
    }
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub trait GenericBus<T: State> {
    fn estop(&self) -> &EstopFlag;
    fn update(&self, state: &mut T);
    fn write(&self, state: &T);
}

// ── Shared context / state types ─────────────────────────────────────────────

pub enum KernelError {
    InitializationFailed,
}


pub trait MaybeDevcie {
    fn start(&'static self, spawner: embassy_executor::Spawner);
}

pub enum DeviceError {

}

pub trait State {}

pub trait Command {}

impl Command for () {}

pub trait Config {}

#[derive(Clone, Copy, Default)]
pub enum Mode {
    Disabled,
    #[default]
    Idle,
    Active,
    Faulted,
}

#[derive(Clone, Copy)]
pub enum Severity {
    Fault,
    Warning,
    Healthy,
}

#[derive(Clone, Default)]
pub struct DeviceState<F: State> {
    pub enabled: bool,
    pub mode:    Mode,
    pub fault:   Option<Fault>,
    pub custom:  F,
}

#[derive(Clone, Default)]
pub struct DeviceConfig<C: Config> {
    pub name: heapless::String<64>,
    pub id: u8,
    pub tick_rate_hz: u32,
    pub custom: C,
}

#[derive(Clone)]
pub struct Fault {
    pub severity: Severity,
    pub message: heapless::String<64>,
}

pub trait Condition {
    fn classify(&self, ctx: &DeviceState<impl State>) -> Fault;
}

pub trait Telemetry {
    fn to_frame(&self, ctx: impl State) -> TelemetryFrame;
}

pub struct TelemetryFrame {
    pub device_id: u8,
    pub mode: Mode,
    pub fault: Option<Fault>,
    pub fields: heapless::Vec<TelemetryField, 8>,
}

pub struct TelemetryField {
    pub name: &'static str,
    pub value: f32,
}

#[allow(async_fn_in_trait)]
pub trait Backend {
    type Output: Copy;
    type Condition: Condition;
    type Config;
    type Error;

    async fn init(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    async fn tick(&mut self) -> Self::Output;
    async fn config(&mut self, config: Self::Config) -> Result<(), Self::Error>;
}

//make kernel backend size scalable
pub trait Kernel {
    type Config: Config;
    type State: State + Copy;

    fn init(&mut self, config: &Self::Config) -> Result<(), KernelError>;

    fn feedback(&self) -> Self::State;

    fn tick(&mut self);

}

pub trait Device {
    type Kernel: Kernel;
    type Command: Command;

    fn tick(&mut self, dt: fugit::Duration<u32, 1, 1000>);
    fn kernel(&mut self) -> &mut Self::Kernel;
    fn execute(&mut self, cmd: Self::Command);
}


pub trait Middleware<State, Config, Command> {
    fn process(&mut self, state: &mut State, config: &Config);
    fn command(&mut self, cmd: Command, state: &mut State, config: &Config);
}

#[derive(Default)]
pub struct NoMiddleware;

impl<S, C, Cmd> Middleware<S, C, Cmd> for NoMiddleware {
    #[inline(always)]
    fn process(&mut self, _state: &mut S, _config: &C) {}
    fn command(&mut self,_: Cmd, _State: &mut S, _config: &C) {}
}


