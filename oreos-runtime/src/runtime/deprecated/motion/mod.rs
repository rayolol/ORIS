//! Motion planning algorithms — stubbed for future development.
//!
//! Currently used only by deprecated code in `runtime::deprecated`.
//! Full implementation pending refactor to macro-based device system.

pub(crate) mod trapezoidal;

pub(crate) use trapezoidal::{MotionPlanner, MotionPlanners, MotionSetpoint, Trapezoidal};
