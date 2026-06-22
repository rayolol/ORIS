use enum_dispatch::enum_dispatch;

use crate::motion::MotionPlanners;
use crate::runtime::kernel::{KernelConfig, KernelState};

#[enum_dispatch]
pub trait ApplyCommand {
    fn apply(&self, motion: &mut MotionPlanners, state: &KernelState, config: &KernelConfig);
}


#[enum_dispatch(ApplyCommand)]
pub enum ActuatorCommand {
    Position(PositionCommand),
    Home(HomeCommand),
}

// each command is a plain struct carrying its arguments
pub struct PositionCommand {
    pub target: f32,
}

pub struct HomeCommand;

impl ApplyCommand for PositionCommand {
     fn apply(&self, motion: &mut MotionPlanners, state: &KernelState, config: &KernelConfig) {
        motion.plan(config, state, self.target);
    }
}

impl ApplyCommand for HomeCommand {
    fn apply(&self, _motion: &mut MotionPlanners, _state: &KernelState, _config: &KernelConfig) {
        todo!()
    }
}
