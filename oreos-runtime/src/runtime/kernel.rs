use crate::hal::{Kernel, KernelError, State, Config, GenericBus};


#[derive(Clone, Copy, Default)]
pub struct KernelState {
    pub current_pos_rad: f32,
    pub current_vel_rad_s: f32,
    pub homed: bool,
    pub stalled: bool,
    pub temp_warning: bool,

    pub target_vel_rad_s: f32,
}


#[derive(Clone, Copy, Default)]
pub struct KernelConfig {
    pub max_vel_rad_s:   f32,
    pub max_accel_rad_s2: f32,
}


impl Config for KernelConfig {}
impl State for KernelState {}


pub struct ActuatorKernel<B: GenericBus<KernelState> + 'static> {
    state:    KernelState,
    bus:       B,
    updaters: heapless::Vec<fn(&B, &mut KernelState), 8>,
    writers: heapless::Vec<fn(&B, &KernelState), 8>,
}

impl<B: GenericBus<KernelState> + 'static> ActuatorKernel<B> {
    pub fn new(bus:  B) -> Self {
        Self {
            bus,
            state:    KernelState::default(),
            updaters: heapless::Vec::new(),
            writers: heapless::Vec::new()
        }
    }

    pub fn register_updater(&mut self, f: fn(&B, &mut KernelState)) {
        let _ = self.updaters.push(f).ok();
    }

    pub fn register_writer(&mut self, f: fn(&B, &KernelState)) {
        let _ = self.writers.push(f).ok();
    }
}

#[maybe_async::sync_impl]
impl<B: GenericBus<KernelState> + 'static> Kernel for ActuatorKernel<B> {
    type Config = KernelConfig;
    type State  = KernelState;

    fn init(&mut self, _config: &Self::Config) -> Result<(), KernelError> {
        // DEMO: nothing to configure at runtime — limits are baked into KernelConfig
        // at construction time via KernelConfig::from_steps().
        Ok(())
    }

    fn feedback(&self) -> Self::State {
        self.state
    }

    fn tick(&mut self) {
        if self.bus.estop().is_set() {
            return;
        }
       
    }
}
