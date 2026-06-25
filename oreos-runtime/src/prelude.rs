//! Prelude for frictionless imports of macros, traits, and bus primitives.
//!
//! This module provides a unified namespace for:
//! - **Macros** from oreos-macros for deriving State, Config, and implementing Kernel/Device/Bus
//! - **HAL traits** for Device, Kernel, State, Config abstractions
//! - **Bus primitives** for inter-component communication (Lane, EstopFlag, GenericBus)
//! - **Bus implementations** (FastLane for low-latency, SlowLane for reliability)
//! - **Device wrappers** (DeviceState, DeviceConfig)
//!
//! # Example
//!
//! ```ignore
//! use robot_arm_firmware_v2::prelude::*;
//!
//! #[derive(State)]
//! struct MyState { }
//!
//! #[derive(Config)]
//! struct MyConfig { }
//!
//! #[derive(GenericBus)]
//! struct MyBus {
//!     #[state]
//!     state: MyState,
//!     estop: EstopFlag,
//!     #[updates(MyState::field => Data::field)]
//!     lane: FastLane<Data>,
//! }
//!
//! #[derive(Kernel)]
//! struct MyKernel {
//!     #[state]
//!     state: MyState,
//!     #[config]
//!     config: MyConfig,
//!     #[bus]
//!     bus: MyBus,
//! }
//!
//! #[create(Device)]
//! struct MyDevice {
//!     #[kernel]
//!     kernel: MyKernel,
//!     #[state]
//!     state: DeviceState<MyState>,
//!     #[config]
//!     config: DeviceConfig<MyConfig>,
//! }
//! ```

// ── Macros from oreos-macros ────────────────────────────────────────────────
pub use oreos_macros::{
    Command, Config, GenericBus, Kernel, Middleware, State, app, create, devices, make_transport,
    middleware,
};

// ── External crates for embedded environment ──────────────────────────────────
#[doc(hidden)]
pub use static_cell::{self, StaticCell};

#[doc(hidden)]
pub use embassy_executor;

#[doc(hidden)]
pub use embassy_stm32;

#[doc(hidden)]
pub use embassy_sync;

#[doc(hidden)]
pub use embassy_time;

#[doc(hidden)]
pub use cortex_m_rt;

#[doc(hidden)]
pub use defmt;

// ── HAL traits and types ────────────────────────────────────────────────────
pub use crate::hal::{
    Backend,
    Command,
    Condition,
    // Traits
    Config as ConfigTrait,
    Device,
    // Types
    DeviceConfig,
    DeviceState,
    EstopFlag,
    Fault,
    GenericBus as GenericBusTrait,
    Kernel as KernelTrait,
    KernelError,
    // Bus primitives (now in HAL)
    Lane,
    MaybeDevcie,
    Middleware,
    Mode,
    Severity,
    State as StateTrait,
};

// -- Utils -------------------------------------------------------------------

// ── Bus implementations from transport ──────────────────────────────────────
pub use crate::transport::{
    bus::{FastLane, SlowLane},
    i2c::*,
    spi::*,
    traits::{PacketRequest, PacketResponse, TransportClient, TransportServer},
    uart::*,
};
