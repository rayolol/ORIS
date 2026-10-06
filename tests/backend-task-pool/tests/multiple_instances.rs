use core::sync::atomic::{AtomicU8, Ordering};
use oreos::prelude::*;

static INITIALIZED: AtomicU8 = AtomicU8::new(0);

// The test polls the raw executor explicitly after spawning its two tasks.
#[unsafe(export_name = "__pender")]
fn pender(_: *mut ()) {}

#[defmt::global_logger]
struct TestLogger;

unsafe impl defmt::Logger for TestLogger {
    fn acquire() {}
    unsafe fn flush() {}
    unsafe fn release() {}
    unsafe fn write(_: &[u8]) {}
}

defmt::timestamp!("");

#[defmt::panic_handler]
fn defmt_panic() -> ! {
    panic!("defmt assertion failed")
}

#[derive(Clone, Copy, Default)]
pub struct TestState;
impl StateTrait for TestState {}

#[derive(Clone, Default)]
pub struct TestConfig;
impl ConfigTrait for TestConfig {}

pub struct KernelStorage;
impl KernelStorage {
    pub const fn new() -> Self {
        Self
    }
}

pub struct TestKernel {
    state: TestState,
}

impl KernelTrait for TestKernel {
    type State = TestState;
    type Config = TestConfig;
    type Storage = KernelStorage;

    fn init(&mut self, _: &TestConfig) -> Result<(), KernelError> {
        Ok(())
    }
    fn feedback(&self) -> TestState {
        self.state
    }
    fn tick(&mut self) {}
}

pub struct TestCondition;
impl Condition for TestCondition {
    fn classify(&self, _: &DeviceState<impl StateTrait>) -> Fault {
        unreachable!("this fixture does not classify faults")
    }
}

pub struct TestBackend(u8);
impl Backend for TestBackend {
    type Output = ();
    type Config = ();
    type Error = ();
    type Condition = TestCondition;

    async fn init(&mut self, _: ()) -> Result<(), ()> {
        INITIALIZED.fetch_or(1 << self.0, Ordering::SeqCst);
        Ok(())
    }
    async fn tick(&mut self) {
        core::future::pending::<()>().await;
    }
    async fn config(&mut self, _: ()) -> Result<(), ()> {
        Ok(())
    }
}

type __DeviceCommand = ();

#[create(Device)]
pub struct TestDevice {
    #[kernel]
    kernel: TestKernel,
    #[state]
    state: DeviceState<TestState>,
    #[config]
    config: DeviceConfig<TestConfig>,
    #[backend(tick_rate: "20", pool_size: 2)]
    backend: TestBackend,
}

#[devices]
struct TestDevices {
    #[device]
    first: TestDevice,
    #[device]
    second: TestDevice,
}

fn device(id: u8) -> TestDevice {
    TestDevice::new(
        TestBackend(id),
        TestKernel { state: TestState },
        DeviceState::default(),
        DeviceConfig::default(),
    )
}

#[test]
fn both_instances_start_and_a_third_exceeds_capacity() {
    let executor = Box::leak(Box::new(embassy_executor::raw::Executor::new(
        core::ptr::null_mut(),
    )));
    let _ctx = devices::__init_devices__(
        executor.spawner(),
        Devices {
            first: device(0),
            second: device(1),
        },
    );

    // No reentrant polling or other executor thread exists in this fixture.
    unsafe { executor.poll() };
    assert_eq!(INITIALIZED.load(Ordering::SeqCst), 0b11);

    let third = Box::leak(Box::new(TestBackend(2)));
    assert!(matches!(
        __BACKEND_TASK__(third),
        Err(embassy_executor::SpawnError::Busy)
    ));
}
