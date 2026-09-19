# OREOS Framework — Documentation Draft

> **Status: draft.** This documents the framework as it actually behaves today,
> verified by reading the macro expansions and trait definitions directly — not
> the aspirational end state. Known gaps are called out inline as **Known
> limitation** so this stays trustworthy as the framework changes. See
> `oreos-cli`'s fix list (tracked separately) for what's planned to close them.

OREOS is a node-driven robotics firmware framework for Rust + Embassy. It is composed of three crates:



| Crate | Role |
|---|---|
| `oreos-runtime` | The library: HAL traits, bus primitives, drivers, motion planning. `no_std`, runs on target hardware. |
| `oreos-macros` | Proc-macros that wire user structs into the HAL traits (`#[create(Device)]`, `#[derive(Kernel)]`, etc.) and generate the Embassy bootstrap (`#[app]`, `#[devices]`). |
| `oreos-cli` (`ordl`) | A standalone host-side tool that reads/writes a project's `OREOS.toml` and scaffolds new devices/backends/middleware as Rust source. |

## 1. Architecture

```
   Host (ROS / control software)
            │
            ▼
     Control Node (MCN or LCN) (microcontroller)
            │
            ▼
          Device            ← #[create(Device)]
            │
            ▼
          Kernel             ← #[derive(Kernel)]   (owns the Bus)
            │
            ▼
           Bus               ← #[derive(GenericBus)]  (routes State ⇄ Lanes)
            │
            ▼
        Backend(s)           ← implements `hal::Backend` by hand
```

- **Control Node**: either an **MCN** (Motor Control Node — drives actuators)
  or an **LCN** (Logic Control Node, sensors, screens, anything that isn't
  motor control). Same code shape; the distinction is purely semantic.

- **Device**: one controlled unit (a motor, a sensor hub, a screen).
  Owns exactly one **Kernel** and any number of **Backends**, any numbers of middlewares **Middleware**.

- **Kernel**: owns the device's `State`, `Config`, and `Bus`.

- **Bus**: the GenericBus type. Holds the device's live `State` plus a set of
  `Lane`s (typed channels) that route individual state fields to/from
  backends via `#[route(State::field <= Lane::field)]` / `=>` attributes.

- **Backend**: the part that talks to real hardware (a stepper driver, an
  LED, a sensor). Implements `hal::Backend` directly — there is currently no
  derive macro for it (see fix list).

- **Middleware**: optional command dispatcher for a device. Receives
  `#[create(Device)]`'s `execute(cmd)` calls and routes them to handler
  functions tagged `#[on(MyCommand::Variant)]`.

## 2. `oreos-runtime`

### 2.1 Module layout


hal:        trait definitions: Device, Kernel, GenericBus, Backend, Middleware,
transport:  UART/SPI/I2C peripheral wrappers, FastLane/SlowLane bus
            implementations, seqlock, host<->MCN transport framing


### 2.2 Core traits (`hal::*`)

```rust
pub trait State {}
pub trait Config {}
pub trait Command {}

pub trait Lane<T> {
    fn write(&self, data: T);
    fn read(&self) -> Option<T>;
}

pub trait GenericBus<T: State> {
    fn estop(&self) -> &EstopFlag;
    fn update(&self, state: &mut T);   // pull lane data into state
    fn write(&self, state: &T);        // push state out to lanes
}

pub trait Kernel {
    type Config: Config;
    type State: State + Copy;
    fn init(&mut self, config: &Self::Config) -> Result<(), KernelError>;
    fn feedback(&self) -> Self::State;
    fn tick(&mut self);
}

pub trait Backend {
    type Output: Copy;
    type Condition: Condition;
    type Config;
    type Error;
    async fn init(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    async fn tick(&mut self) -> Self::Output;
    async fn config(&mut self, config: Self::Config) -> Result<(), Self::Error>;
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
```

You will essentially never implement `Device`, `Kernel`, or `GenericBus` by
hand — `oreos-macros` derives them for you from a plain struct (§3). You
**do** implement `Backend` by hand for each piece of hardware you support;
`drivers::{StepperBackend, LedBackend}` are the reference implementations to
copy from.

`DeviceState<T: State>` and `DeviceConfig<T: Config>` are generic wrappers
every device's user-defined state/config get wrapped in automatically. They
carry framework-level bookkeeping (`enabled`, `mode`, `fault`) alongside your
`custom: T` payload.

### 2.3 Crates `oreos-runtime` re-exports for you

Macro-generated code (and a couple of framework helper macros) reference
`defmt`, `embassy_time`, and `fugit` by name. Rust only resolves a bare
`some_crate::...` path against a crate's **own direct dependencies** — so if
your firmware crate only depends on `oreos-runtime` (which depends on those
three), a bare path to them fails with "could not find `defmt` in the list of
imported crates", even though `defmt` is clearly part of the build.

The fix baked into the framework: `oreos-runtime` re-exports them at its
crate root (`pub use defmt; pub use embassy_time; pub use fugit;`), and all
macro-generated code reaches them via `::Oreos::defmt::...` etc. instead of a
bare path. Anything **you** write that needs one of these three directly
(e.g. a custom `Backend` impl using `fugit::Duration`) should go through the
same re-export — `oreos::defmt::info!(...)`, `oreos::embassy_time::Timer`,
`oreos::fugit::Duration` — rather than adding `defmt`/`embassy-time`/`fugit`
as direct dependencies of your own crate. This keeps version selection for
those three crates centralized in `oreos-runtime`.

A small consequence: `install_defmt_timestamp!()` (exported from
`oreos-runtime`, see `src/utils.rs`) wraps the standard
defmt-panic-handler-plus-timestamp boilerplate every consumer needs once.
Being a `macro_rules!` macro (not a proc-macro), it resolves its internal
`defmt`/`embassy_time` references via `$crate::` rather than `::Oreos::`,
which is the idiomatic way for a `macro_rules!` macro to always point back at
its defining crate regardless of call site. Typical `main.rs`:

```rust
pub use Oreos as oreos;
use oreos::prelude::*;

// expands to: a `#[defmt::panic_handler]` fn that loops forever, plus the
// `TIME_INITIALIZED` static + `defmt::timestamp!` wiring. Pass an expression
// to run inside the panic loop instead of an empty one, e.g.
// `oreos::install_defmt_timestamp!(led.set_high());`
oreos::install_defmt_timestamp!();
```

`panic-probe` and `defmt-rtt` need no such treatment — `#[panic_handler]` and
defmt's global logger are linker-level hooks, not path-resolved, so merely
having them anywhere in the dependency graph (even only transitively, via
`oreos-runtime`) is enough. No `use panic_probe as _;` /
`use defmt_rtt as _;` lines are required in consumer code.

## 3. `oreos-macros`

All macros are re-exported through `oreos_runtime::prelude`, so user code
only ever writes `use oreos::prelude::*;` (or `oreos_runtime::prelude::*` if
not aliased — see fix list re: the `Oreos` crate name).

| Macro | Kind | Purpose |
|---|---|---|
| `#[derive(State)]` | derive | Marks a struct as a device's custom state payload. Generates a `__DeviceState` type alias + `impl hal::State`. |
| `#[derive(Config)]` | derive | Same, for config. Generates `__DeviceConfig` + `impl hal::Config`. |
| `#[derive(Command)]` | derive | Turns an enum into a device's command type. Generates `__DeviceCommand` + `impl hal::Command`, and registers the enum so `#[on(...)]` (below) can look up its variants later in the same compile. |
| `#[derive(GenericBus)]` (`#[state]`, `#[route(...)]`) | derive | Builds a static-allocated bus: a `::new(...)` constructor, `impl hal::GenericBus`, and one accessor method per `#[route]`-tagged lane field. `#[route(State::f <= Lane::f)]` means "pull lane → state on update"; `=>` means "push state → lane on write". |
| `#[derive(Kernel)]` (`#[state]`, `#[config]`, `#[bus]`) | derive | Builds `::new(state, config, bus) -> Self` and `impl hal::Kernel`. Requires exactly one field each tagged `#[state]`, `#[config]`, `#[bus]`. |
| `#[derive(Middleware)]` | derive | Implements `hal::Middleware` for the struct, delegating to a `callback_match` method — which only exists once you also apply `#[middleware]` below. |
| `#[middleware]` (on an `impl` block, with `#[on(Command::Variant)]` per fn) | attribute | Generates `callback_match`, which matches every `#[on(...)]`-tagged function against the named command variant. **A `#[derive(Command)]` enum with that variant must already be visible in the same compilation** (see §5 — this lookup is a known fragility point). |
| `#[create(Device)]` (`#[kernel]`, `#[state]`, `#[config]`, `#[backend]`, `#[middleware]`) | attribute | The big one — see below. |
| `#[devices] struct Dev { ... }` | attribute | Declares the application's top-level device/peripheral list. Generates a `Devices` struct, a `Context`/`ContextView` dependency-injection type, and `__init_devices__`. Fields tagged `#[device]` get `.start(spawner)` called automatically (anything implementing `MaybeDevice`); `#[shared]` fields are wrapped for multi-task access; everything else is exclusively owned. |
| `#[app(hal_crate = ...)] mod app { ... }` | attribute | Generates the `#[embassy_executor::main]` entrypoint. Exactly one `#[init(config = ...)]` async fn must return `Devices`. Each `#[loop_(rate = N)]` async fn becomes its own spawned Embassy task running every `N` ms, with `ctx: Context` rewritten so the IDE sees real device types. |

### 3.1 `#[create(Device)]` in depth

Given:

```rust
#[create(Device)]
struct ArmJoint {
    #[kernel]    kernel: ArmJointKernel,
    #[state]     state: DeviceState<ArmJointState>,
    #[config]    config: DeviceConfig<ArmJointConfig>,
    #[backend]   stepper: StepperBackend<Output<'static>, Output<'static>, Output<'static>, FastLane<StepperData>>,
    #[middleware] middleware: ArmJointMiddleware,
}
```

the macro:

1. Strips the field-role attributes (`#[kernel]`, `#[state]`, ...) from the
   final struct definition.
2. Rewrites every `#[backend]` field's type to
   `UnsafeCell<Option<OriginalType>>` so the backend can be moved out into a
   `'static` later, and generates a `static StaticCell` + an
   `#[embassy_executor::task]` loop that calls `backend.tick()` every 10ms
   for it.
3. If no field is tagged `#[middleware]`, inserts a hidden
   `__NO_MIDDLEWARE__: NoMiddleware` field instead — middleware is optional.
4. Emits `impl ArmJoint { pub fn new(...) -> Self }`, `impl hal::Device for
   ArmJoint`, and `impl hal::MaybeDevice for ArmJoint` (the `start()` that
   moves each backend into its static and spawns its task).

**Only one field may carry each of `#[kernel]`, `#[state]`, `#[config]`,
`#[middleware]`** — multiple are silently overwritten (last one wins), not
rejected. `#[backend]` is the one role that supports multiple fields.

## 4. `oreos-cli` (`ordl`)

A standalone host-side binary (separate Cargo workspace from
`oreos-runtime`/`oreos-macros` — it builds for your machine, not the target
MCU). Source of truth is `OREOS.toml` in the current directory.

```
ordl new device              # interactively append a Device to OREOS.toml
ordl new backend --from DEV   # append a Backend to an existing device
ordl new middleware --from DEV
ordl show [--device NAME]     # pretty-print OREOS.toml (or one device)
ordl generate --device NAME --output ./src   # emit Rust source for a device
ordl sync                     # scan target/.oreos/tmp for code-side metadata
                               # and reconcile it back into OREOS.toml
```

`generate` writes one directory per device (`<output>/<snake_case device
name>/`), containing `kernel.rs`, `device.rs`, one file per backend/middleware,
and a `mod.rs` declaring them all. It does **not** wire that module into your
crate — you still add `mod <device_name>;` to whatever file owns it.

`sync` reads the JSON metadata files macros write under `target/.oreos/tmp`
(see §5) and adds anything found in code but missing from `OREOS.toml`
(`AddToToml`), flags anything in code that isn't claimed by a device
(`WarnDangling`), and refuses to delete a device that still exists in
`OREOS.toml` but has disappeared from code (`WarnDeleteBlocked` — sync never
deletes for you).

## 5. Known limitations / fragile areas

These aren't bugs in the sense of "broken right now" but are worth knowing
before relying on them:

- **`#[on(...)]` command lookup is compile-order-dependent.** `oreos-macros`
  keeps an in-process `HashMap` (`registry::store`/`fetch`) that a
  `#[derive(Command)]` writes to and a later `#[middleware]` block reads
  from. This only works within a single proc-macro-server process for one
  crate compile; incremental rebuilds that skip re-expanding the
  `#[derive(Command)]` site (because that file didn't change) can leave the
  registry empty for an otherwise-unchanged middleware file. A clean rebuild
  always works; incremental rebuilds are the risk case.

## 6. Glossary

| Term | Meaning |
|---|---|
| MCN | Motor Control Node — a control node responsible for actuators |
| LCN | Logic Control Node — a control node responsible for non-motor logic (sensors, displays...) |
| Lane | A typed channel routing one field of state to/from a backend |
| Bus | The `GenericBus` — owns state + lanes + the e-stop flag for one device |
| Backend | The hardware-facing implementation behind a device (driver, sensor, etc.) |
