//! Deprecated runtime components — kept for reference but no longer actively used.
//!
//! These modules demonstrate early design patterns and are maintained for documentation purposes.
//! New code should use the macro-based device system (State, Config, Kernel, create macros).

pub mod actuator;
pub mod command;
pub mod backend;

pub use actuator::{ActuatorNode, ActuatorConf, ActuatorConfig};
pub use command::{ActuatorCommand, ApplyCommand, PositionCommand, HomeCommand};
pub use backend::BackendContainer;
