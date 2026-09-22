# Building a system with OREOS

This guide describes the current, incremental workflow: start with one working
node, introduce devices only where they create a useful boundary, and expand
the model as the firmware proves what the framework needs.

For component semantics, see [Framework model](FRAMEWORK.md). For unstable or
missing behavior, see [Limitations and roadmap](LIMITATIONS_AND_ROADMAP.md).

## 1. Start by drawing boundaries

Use three levels:

1. **System**: the whole machine or robot.
2. **Control node**: one MCU and firmware image.
3. **Device**: one active subsystem inside a node.

Do not start by turning every file, pin, or physical part into a device.

### When something should be its own control node

Use a separate node when one or more of these are true:

- it runs on a different MCU;
- it must continue or fail independently;
- it owns physically separate I/O that cannot be driven by the existing MCU;
- it has a distinct real-time or safety boundary;
- communication with it naturally crosses a transport such as CAN, UART, or
  Ethernet;
- it is deployed, updated, or tested as a separate firmware image.

`OREOS.toml` currently describes one node, not the whole multi-node system.

### When something should be a device

A subsystem is a strong device candidate when most of these statements are true:

- it has meaningful state of its own;
- it has configuration or calibration;
- it accepts commands;
- it performs periodic control;
- it owns one or more hardware-facing backends;
- it has a fault, enable/disable, or lifecycle boundary;
- it can be tested as a unit independently of the rest of the machine.

Create a device because the boundary improves ownership and reasoning—not just
because a physical part exists.

### When something should not be a device

Keep it as a normal Rust type, runtime resource, or backend when it is:

- a stateless helper or algorithm such as a PID implementation;
- a raw pin or peripheral used directly by application initialization;
- a passive data structure;
- a queue, transport handle, or synchronization primitive;
- a tiny hardware adapter owned completely by another device;
- a status LED with no independent lifecycle or control policy.

### Example: reflow oven

| Part | Likely scope | Reason |
|---|---|---|
| Top heating zone | `TopHeater` device | Independent target, feedback, control, output, and fault behavior. |
| Bottom heating zone | Separate device | Same reason if it is controlled and faulted independently. |
| Dedicated top thermocouple | Backend inside `TopHeater` | It exists to provide that device's feedback. |
| Shared chamber safety sensor | Separate monitor device or node-level service | Its data and faults affect several devices. |
| Heater PWM/SSR output | Backend inside the heater device | Hardware-facing mechanism, not machine policy. |
| PID calculation | Kernel logic or helper type | Algorithmic policy, not an independently active subsystem. |
| Status LED | Plain runtime resource | Usually no state machine or fault boundary is needed. |
| User interface | Device if active and stateful | It may have commands, state, periodic updates, and hardware backends. |

If two parts must always change, initialize, stop, and fail together, begin with
one device. Split them later when independent ownership becomes valuable.

## 2. Plan one node

Before running the CLI, write down:

- node name and responsibility;
- MCU/board and Rust target;
- runtime resources needed during startup;
- candidate devices;
- each device's state, configuration, commands, control rate, and fault boundary;
- which physical interfaces each device ultimately owns.

The current tool does not validate board pins or peripheral conflicts. Keep a
temporary hardware allocation table in the firmware repository until the
two-stage hardware configuration model is implemented.

Example:

| Resource | Owner | Binding |
|---|---|---|
| Heater output | `TopHeater.HeaterControl` | Timer/PWM channel or GPIO |
| Temperature input | `TopHeater.TemperatureSense` | SPI/ADC peripheral |
| Status LED | Node runtime | GPIO |

## 3. Create the firmware runtime

The runtime is application-specific. OREOS generates its shape, but the
application owns initialization and scheduling.

The current runtime concepts are:

```rust
#![no_std]
#![no_main]

use oreos::prelude::*;
use embassy_stm32::{Config, Peripherals};

mod top_heater;

oreos::install_defmt_timestamp!();

#[devices]
struct Dev {
    #[device]
    top_heater: top_heater::device::TopHeater,
    // Plain resources can live beside OREOS devices.
}

#[app(hal_crate = embassy_stm32)]
mod app {
    #[init(config = Config::default())]
    async fn setup(p: Peripherals, _spawner: Spawner) -> Devices {
        // Application code:
        // 1. configure hardware from p;
        // 2. construct lanes and the device bus;
        // 3. construct kernel state/config;
        // 4. construct backends;
        // 5. construct TopHeater;
        // 6. return it in Devices.

        Devices { top_heater }
    }

    #[loop_(rate = 10)]
    async fn control(mut ctx: Context) {
        // Starting a #[device] starts its backend tasks.
        // The application still chooses when to call the device tick.
        ctx.top_heater.tick(oreos::fugit::Duration::millis(10));
    }
}
```

The constructor details depend on the device and board. They belong in
`#[init]` or application-owned constructor helpers, not in framework macros.

## 4. Describe a device

From the firmware project root:

```text
ordl new device
```

The current command asks for:

- device name;
- state type name;
- config type name;
- bus name.

It updates `OREOS.toml` only. For example:

```toml
name = "reflow-control"
unresolved = []

[[devices]]
name = "TopHeater"
middleware = []

[devices.state]
name = "HeaterState"

[devices.config]
name = "HeaterConfig"

[devices.kernel]
name = "TopHeaterKernel"

[devices.kernel.bus]
name = "HeaterBus"
lane_type = "sync"
bound = "unbounded"
```

Names currently drive generated Rust identifiers and filenames. Prefer
PascalCase type names without redundant suffixes. The bus template adds `Bus`,
so entering `Heater` currently produces `HeaterBus` while entering `HeaterBus`
produces `HeaterBusBus`.

## 5. Add backends and middleware to the model

Add a backend:

```text
ordl new --from TopHeater backend
Backend name: HeaterControl
```

The position of `--from` is important in the current CLI: it belongs to `new`,
before the `backend` subcommand.

Add middleware only when the device needs command dispatch:

```text
ordl new --from TopHeater middleware
Middleware name: HeaterCommands
```

These commands update `OREOS.toml`. They do not create component files.

## 6. Generate the scaffold

Generate the full device once:

```text
ordl generate --device TopHeater
```

The output is:

```text
src/top_heater/
├── mod.rs
├── device.rs
├── kernel.rs
└── heater_control_backend.rs
```

Then expose it from the crate root:

```rust
mod top_heater;
```

### Regeneration warning

`ordl generate --device TopHeater` currently rewrites `kernel.rs`,
`device.rs`, every known backend/middleware file, and `mod.rs`. It does not
merge user edits.

Treat full generation as initial scaffolding until safe regeneration exists.
Before rerunning it, inspect or commit your work.

### Generate only a newly registered backend

After adding a backend to `OREOS.toml`:

```text
ordl generate --device TopHeater --backend HeaterControl
```

This generates only:

```text
src/top_heater/heater_control_backend.rs
```

The backend-only command refuses to overwrite an existing file. It does not
currently update `device.rs` or `mod.rs`, so add the module, import, and
`#[backend]` field yourself when adding a backend to an already edited device.

There is not yet an equivalent safe middleware-only generator.

## 7. Implement the device from the inside out

The most reliable implementation order is data first, hardware last.

### Step 1: state

Put values that change during control in the state payload:

```rust
#[derive(State, Clone, Copy)]
pub struct HeaterState {
    pub target_celsius: f32,
    pub measured_celsius: f32,
    pub output_duty: f32,
}
```

Keep this type `Copy` because the current kernel/device flow copies it.

### Step 2: configuration

Put device policy and calibration in configuration:

```rust
#[derive(Config)]
pub struct HeaterConfig {
    pub maximum_celsius: f32,
    pub control_period_ms: u32,
}
```

Do not put concrete MCU pins here. Hardware binding is currently performed in
application initialization and will later move to the hardware configuration
layer.

### Step 3: commands

Define what callers may request:

```rust
#[derive(Command)]
pub enum TopHeaterCommand {
    Enable,
    Disable,
    SetTarget(f32),
}
```

Command/middleware handler discovery is still under migration. Defining the
command type works; rely on `#[on(...)]` handlers only after the Sema work
listed in the roadmap lands.

### Step 4: bus and lanes

Define typed boundaries between control state and backend data. A lane should
carry data with one clear producer/consumer relationship.

Use incoming routes for measurements and outgoing routes for actuator requests.
Construct the lanes and bus in application initialization.

### Step 5: kernel

The kernel is where device policy belongs: state evolution, control laws,
limits, and coordination between incoming feedback and outgoing requests.

The current derive generates fixed update/write behavior. If the device needs
custom control calculations, keep them in explicit helper methods called by the
application or wait for the kernel customization surface to be expanded; do not
assume the derive currently invents control logic.

### Step 6: backends

A backend should translate between typed device data and one hardware mechanism.
It should not decide machine-wide policy.

Implement:

- `Output`: data returned from a tick;
- `Condition`: backend health classification;
- `Config` and `Error`;
- `init`, `tick`, and `config`.

Remember that generated backend tasks currently call only `tick` at a fixed
10 ms interval. Call initialization/configuration explicitly where needed and
connect outputs through lanes or other explicit channels.

### Step 7: middleware, only if needed

Middleware belongs between external commands and device state/configuration.
Do not introduce it merely to run the control loop; that is the kernel's role.

## 8. Assemble the device

Application initialization performs the actual ownership transfer:

```text
MCU peripherals
    -> concrete hardware drivers
    -> backend instances
lanes
    -> bus
state + config + bus
    -> kernel
backend(s) + kernel + DeviceState + DeviceConfig
    -> device
device
    -> Devices returned by #[init]
```

This is intentionally application code. The framework cannot choose pin
assignments, calibration, safe defaults, or runtime policy on behalf of the
firmware author.

## 9. Schedule and coordinate the node

Use application loops for node-level behavior:

- tick each device at the rate its control policy requires;
- execute commands received from transports or higher-level logic;
- coordinate several devices;
- implement system-level safety that is broader than one device;
- publish telemetry.

Do not hide node-level coordination inside one device backend. Backends should
remain hardware-facing and reusable.

The current `#[loop_(rate = N)]` rate is a delay in milliseconds, not hertz.
The `priority` argument is parsed but not applied yet.

## 10. Grow from one node to a system

For every additional node:

1. create a separate firmware project;
2. define that node's hardware ownership;
3. scope its devices independently;
4. expose cross-node behavior through an explicit transport;
5. assign stable node/device identities at the ORIS layer;
6. test loss, reset, and fault behavior at the transport boundary.

Today, the multi-node system graph and transport bindings are primarily manual.
Future ORIS tooling should compose node manifests without moving
application-specific initialization into the framework.

## 11. Change and deletion workflow

Prefer additive changes:

1. update `OREOS.toml`;
2. generate only the new component when a safe component generator exists;
3. wire it into existing source manually;
4. compile before adding behavior;
5. commit before any full regeneration.

Deletion is deliberately manual. Remove references in code, verify the build,
then remove the corresponding manifest entry and file. `ordl sync` does not
delete components for you.

## 12. A device is complete when

- its boundary and ownership are clear;
- state, configuration, and commands have distinct responsibilities;
- hardware is owned by backends or explicit application resources;
- bus directions and lane ownership are documented;
- initialization values are supplied by the application;
- backend startup and device ticking are both scheduled intentionally;
- safe defaults and fault behavior are defined;
- the node compiles and the device can be tested independently;
- its `OREOS.toml` description and source agree.
