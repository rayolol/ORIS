#![no_std]

// ── Generic, reusable layer ───────────────────────────────────────────────────
pub mod hal;        // hardware abstraction traits + shared motor types + bus primitives
pub mod drivers;    // concrete HAL implementations (StepperBackend, TMC2209)
pub mod transport;  // bus implementations, seqlock, UART/SPI/I2C peripheral wrappers

pub(crate) mod motion; // motion planning algorithms (Trapezoidal)

// ── Application-specific layer ────────────────────────────────────────────────
pub mod api;        // public command / status / event types
pub mod config;     // ActuatorConfig, HardwareConfig, NodeConfig
pub mod runtime;    // kernel, bus, transport layers
pub mod core;       // Kernel, Scheduler, Task, etc.

// ── Prelude ───────────────────────────────────────────────────────────────────
pub mod prelude;    // unified imports for macros, traits, and bus primitives

pub use api::{MotorState, SensorState};
pub use drivers::tmc2160::structs::DriverStatus;