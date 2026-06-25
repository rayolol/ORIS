#![no_std]

// ── Generic, reusable layer ───────────────────────────────────────────────────
pub mod drivers; // concrete HAL implementations (StepperBackend, TMC2209)
pub mod hal; // hardware abstraction traits + shared motor types + bus primitives
pub mod transport; // bus implementations, seqlock, UART/SPI/I2C peripheral wrappers
pub mod utils; // utility functions

// ── Prelude ───────────────────────────────────────────────────────────────────
pub mod prelude; // unified imports for macros, traits, and bus primitives

pub use drivers::tmc2160::structs::DriverStatus;

// Re-exported so macro-generated code (and consumer code) can reach these via
// `Oreos::defmt`/`Oreos::fugit`/`Oreos::embassy_time` without needing them as
// direct dependencies — only `oreos-runtime` needs to declare them.
pub use defmt;
pub use embassy_time;
pub use fugit;
