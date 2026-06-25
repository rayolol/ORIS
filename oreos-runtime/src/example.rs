#![no_std]
#![no_main]
mod shared_cell;
use defmt::trace;
use embassy_time::Timer;
use oreos_macros::*;
pub use shared_cell::{Shared, SharedCell};

use defmt_rtt as _;
use embassy_stm32::{
    Config, bind_interrupts,
    gpio::{AnyPin, Level},
    mode::{Async, Mode},
    peripherals::USART3,
    rcc::{APBPrescaler, Pll, PllMul, PllPreDiv, PllSource, Sysclk},
    usart::{self, BufferedUart, Uart},
};
use panic_probe as _;
use static_cell::StaticCell;

use core::sync::atomic::{AtomicBool, Ordering};

static TIME_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn defmt_timestamp_micros() -> u64 {
    if TIME_INITIALIZED.load(Ordering::Relaxed) {
        embassy_time::Instant::now().as_micros()
    } else {
        0
    }
}

defmt::timestamp!("{=u64}", defmt_timestamp_micros());

use embassy_stm32::{Peripherals, gpio::Output, gpio::Speed, peripherals};

use Oreos::{
    drivers::{LedBackend, LedData, StepperBackend, motor::StepperData},
    hal::Backend,
    prelude::*,
};

use core::cell::UnsafeCell;

static RX_BUFFER: StaticCell<[u8; 128]> = StaticCell::new();
static TX_BUFFER: StaticCell<[u8; 128]> = StaticCell::new();

#[derive(Default, State, Clone, Copy)]
struct MyState {
    pub target_speed: i32,
    pub current_speed: i32,
    pub led_enabled: bool,
}

#[derive(Default, Config, Clone, Copy)]
struct MyConfig {
    max_speed: u32,
}

#[derive(GenericBus)]
struct MyBus {
    #[state]
    state: MyState,
    estop: EstopFlag,
    #[route(MyState::target_speed => StepperData::vel_steps_per_s)]
    lane1: FastLane<StepperData>,
    #[route(MyState::led_enabled => LedData::enabled)]
    led_lane: FastLane<LedData>,
}

#[derive(Kernel)]
struct MyKernel {
    #[state]
    state: MyState,
    #[config]
    config: MyConfig,
    #[bus]
    bus: &'static MyBus,
}

#[derive(Command)]
enum MyCommand {
    Enable,
    Disable,
    Blink { count: u32 },
    SetTarget(f32),
    Run { code: i32, address: usize },
}

#[derive(Middleware, Default)]
struct MyMiddleware {}

#[middleware]
impl MyMiddleware {
    #[on(MyCommand::Enable)]
    fn on_enable(state: &mut DeviceState<MyState>, _config: &DeviceConfig<MyConfig>) {
        defmt::info!("middleware: Enable command");
        state.custom.led_enabled = true;
    }

    #[on(MyCommand::Disable)]
    fn on_disable(state: &mut DeviceState<MyState>, _config: &DeviceConfig<MyConfig>) {
        defmt::info!("middleware: Disable command");
        state.custom.led_enabled = false;
    }

    #[on(MyCommand::Blink)]
    fn on_blink(state: &mut DeviceState<MyState>, _config: &DeviceConfig<MyConfig>, blink: Blink) {
        defmt::info!("middleware: Blink command count={}", blink.count);
        defmt::info!("middleware: setting led_enabled=true");
        state.custom.led_enabled = !state.custom.led_enabled;
        defmt::info!(
            "middleware: led_enabled is now {}",
            state.custom.led_enabled
        );
    }

    #[on(MyCommand::SetTarget)]
    fn on_set_target(
        state: &mut DeviceState<MyState>,
        config: &DeviceConfig<MyConfig>,
        target: f32,
    ) {
        defmt::debug!("middleware: SetTarget target={}", target as i32);
    }

    #[on(MyCommand::Run)]
    fn on_run(state: &mut DeviceState<MyState>, config: &DeviceConfig<MyConfig>, run: Run) {
        let Run { code, address } = run;
        defmt::info!("middleware: Run command code={}, address={}", code, address);
    }
}

#[create(Device)]
struct actuator {
    #[kernel]
    kernel: MyKernel,
    #[state]
    state: DeviceState<MyState>,
    #[config]
    config: DeviceConfig<MyConfig>,
    #[backend]
    backend1:
        StepperBackend<Output<'static>, Output<'static>, Output<'static>, FastLane<StepperData>>,
    #[backend]
    led_backend: LedBackend<Output<'static>, FastLane<LedData>>,
    #[middleware]
    middleware: MyMiddleware,
}

#[devices]
struct Dev {
    transport_server: TransportServer<SingleWireUart<BufferedUart<'static>>>,
    transport_client: TransportClient,
    #[device]
    actuator: actuator,
}

#[app(hal_crate = embassy_stm32)]
mod app {
    use Oreos::prelude::*;
    use Oreos::{
        hal::DeviceConfig,
        transport::bus::{FastLane, SlowLane},
    };
    use embassy_executor::Spawner;

    bind_interrupts!(struct Irqs {
        USART3 => embassy_stm32::usart::BufferedInterruptHandler<peripherals::USART3>;
    });

    #[init(config = Config::default())]
    async fn init(p: Peripherals, _: Spawner) -> Devices {
        defmt::info!("======================================");
        defmt::info!("Robot Arm Firmware v2 - Initialization");
        defmt::info!("======================================");

        defmt::info!("init: configuring UART");
        let mut uart_conf = embassy_stm32::usart::Config::default();
        uart_conf.baudrate = 9600;

        let buffered = BufferedUart::new_half_duplex(
            p.USART3,
            p.PB10, // TX pin
            Irqs,
            TX_BUFFER.init([0u8; 128]),
            RX_BUFFER.init([0u8; 128]),
            uart_conf,
            embassy_stm32::usart::HalfDuplexReadback::NoReadback,
        )
        .unwrap();

        let uart = SingleWireUart::new(buffered);

        defmt::info!("init: creating transport layer");
        let (client, server) = make_transport!(uart);

        defmt::info!("init: configuring GPIO pins");
        let step = Output::new(p.PA10, Level::High, Speed::Low);
        let dir = Output::new(p.PA11, Level::High, Speed::Low);
        let en = Output::new(p.PB9, Level::High, Speed::Low);
        let led = Output::new(p.PC13, Level::High, Speed::Low);
        defmt::debug!("init: GPIO pins configured (step=PA10, dir=PA11, en=PB9, led=PC13)");

        defmt::info!("init: creating bus");
        let bus = MyBus::new(
            FastLane::<StepperData>::new(StepperData {
                position_steps: 0,
                vel_steps_per_s: 50,
            }),
            FastLane::<LedData>::new(LedData::default()),
            MyState::default(),
            EstopFlag::new(),
        );
        defmt::debug!("init: bus created with initial stepper velocity=50");

        defmt::info!("init: creating actuator device");
        let act = actuator::new(
            StepperBackend::new(step, dir, en, 100000, bus.lane1()),
            LedBackend::new(led, bus.led_lane()),
            MyKernel::new(MyState::default(), MyConfig { max_speed: 56 }, bus),
            DeviceState::<MyState>::default(),
            DeviceConfig::<MyConfig>::default(),
            MyMiddleware::default(),
        );
        defmt::info!("init: actuator device created successfully");

        defmt::info!("init: initialization complete");
        Devices {
            actuator: act,
            transport_server: server,
            transport_client: client,
        }
    }

    #[loop_(rate = 100u64, priority = 0)]
    async fn loop_task(ctx: Context) {
        defmt::debug!("loop_task: calling device tick");
        ctx.actuator
            .tick(fugit::Duration::<u32, 1, 1000>::from_millis(5));
    }

    #[loop_(rate = 1000u64, priority = 0)]
    async fn command_task(ctx: Context) {
        let now = embassy_time::Instant::now();
        defmt::info!("command_task: executing Blink at time={}", now.as_millis());
        ctx.actuator.execute(MyCommand::Blink { count: 3 });
    }
}
