# OREOS framework model

This document describes the framework as it behaves today. Planned behavior is
kept in [Limitations and roadmap](LIMITATIONS_AND_ROADMAP.md) so application
code is not written against features that do not exist yet.

OREOS is the embedded firmware layer of ORIS. ORIS is the wider interface
standard; OREOS is the Rust/Embassy framework used to build firmware for the
control nodes participating in that system.

## 1. The boundaries

The most important distinction is between the framework and an application.

The framework provides:

- structural traits for devices, kernels, buses, backends, and middleware;
- macros that implement those traits and generate Embassy task/bootstrap code;
- static storage and task startup for declared runtime resources;
- typed state/config wrappers, lanes, transport primitives, and reusable drivers;
- a CLI model and scaffolding workflow.

The firmware application still decides:

- which hardware is initialized and with which board-specific settings;
- the concrete state and configuration values;
- how constructors are called and components are wired together;
- which devices exist on a node;
- when each device kernel is ticked;
- runtime policy, rates, fault responses, and machine-level coordination.

In particular, OREOS does not currently infer application initialization from
the device declaration. Generated `init()`, `config()`, and constructor surfaces
are framework hooks; supplying values and deciding when to call them remains
application-specific code.

## 2. System hierarchy

```text
ORIS system
├── control node A (one firmware image / MCU)
│   ├── OREOS runtime
│   │   ├── ordinary runtime resources
│   │   └── active OREOS devices
│   │       ├── state + configuration + commands
│   │       ├── kernel
│   │       ├── bus and lanes
│   │       ├── zero or more backends
│   │       └── optional middleware
│   └── board-specific initialization
└── control node B
    └── ...
```

### System

A system is the whole robot or machine. It may contain several control nodes.
OREOS does not yet have a complete multi-node system manifest; `OREOS.toml`
currently models one control node.

### Control node

A control node is one deployable firmware image, normally one MCU. The node is
the boundary for `Cargo.toml`, the target configuration, `memory.x`,
`OREOS.toml`, `#[devices]`, and `#[app]`.

MCN (motor-control node) and LCN (logic-control node) are semantic roles. They
do not currently select different generated code.

### Runtime resource

A runtime resource is any value constructed during `#[init]` and returned in
the generated `Devices` container. A resource can be a raw peripheral wrapper,
a service, a queue, a status LED, or an OREOS device.

Not every runtime resource needs to be an OREOS device.

### Device

A device is a cohesive, active subsystem with its own state and lifecycle. A
device is appropriate when a subsystem benefits from a clear control boundary:
it has state to evolve, configuration, commands, periodic control, hardware
backends, or a fault boundary.

A device is not synonymous with a physical component. One device may coordinate
several physical parts, while a small physical part may remain a backend or an
ordinary runtime resource.

## 3. Device components

```text
command ──> middleware ──> DeviceState / DeviceConfig
                              │
application tick ──> device ──┤
                              ▼
                           kernel
                              │
                              ▼
                         bus + lanes
                              │
                     backend task(s)
                              │
                              ▼
                           hardware
```

This diagram is a responsibility map, not fully automatic data wiring. The
application and constructors still connect concrete lanes, backends, and
hardware.

### State

The application defines a small `Copy` payload and derives `State`:

```rust
#[derive(State, Clone, Copy)]
pub struct HeaterState {
    pub target_celsius: f32,
    pub measured_celsius: f32,
    pub duty: f32,
}
```

The device stores it inside `DeviceState<T>`, which adds framework state:
`enabled`, `mode`, and `fault`.

State describes what changes while the device runs. It should not contain
board pin identities or long-lived policy configuration.

### Configuration

The application derives `Config` for device policy that changes less frequently:

```rust
#[derive(Config)]
pub struct HeaterConfig {
    pub maximum_celsius: f32,
}
```

The device stores it inside `DeviceConfig<T>`, alongside a device name, ID, and
tick-rate field. Concrete values remain application-owned.

Board selection and pin/peripheral assignments are not yet represented by this
type; the planned two-stage hardware model is described in the roadmap.

### Command and middleware

`#[derive(Command)]` marks the enum accepted by `Device::execute`.
Middleware is optional. When present, it maps commands to changes in state or
configuration and can process state during a device tick.

The middleware handler discovery path is not complete today. Empty/default
middleware works, but command handlers using `#[on(...)]` are part of the Sema
migration and should be treated as unstable.

### Kernel

The kernel owns the device's control behavior. The current `#[derive(Kernel)]`
implementation:

1. checks the bus e-stop flag;
2. pulls incoming lane data into its state with `bus.update`;
3. pushes state to outgoing lanes with `bus.write`;
4. exposes feedback as a copy of its state.

The generated device copies `DeviceState.custom` into the kernel before the
kernel tick and copies kernel feedback back afterward.

### Bus and lanes

The bus is a device-local typed data boundary. A `#[derive(GenericBus)]` struct
contains:

- one field tagged `#[state]`;
- an `EstopFlag`;
- zero or more lane fields tagged with `#[route(...)]`.

```rust
#[derive(GenericBus)]
pub struct HeaterBus {
    #[state]
    pub state: HeaterState,
    pub estop: EstopFlag,

    #[route(HeaterState::duty => HeaterOutput::duty)]
    pub output: FastLane<HeaterOutput>,

    #[route(HeaterState::measured_celsius <= TemperatureSample::celsius)]
    pub temperature: FastLane<TemperatureSample>,
}
```

`<=` pulls a lane field into state during `update`. `=>` pushes a state field
into lane data during `write`.

### Backend

A backend is hardware-facing behavior owned by a device: GPIO/PWM output,
temperature acquisition, a motor driver, and similar work.

Backends implement `hal::Backend` manually. `#[create(Device)]` moves each
backend into static storage and spawns one Embassy task per backend. The
generated task currently calls `tick()` every 10 ms. It does not automatically
call `Backend::init` or `Backend::config`, and it currently ignores the value
returned by `tick`; the application/backend wiring must handle useful data via
lanes or other explicit channels.

## 4. Node startup and execution flow

### Boot

1. `#[app(hal_crate = ...)]` generates `#[embassy_executor::main]`.
2. The generated main calls `hal_crate::init(config)`.
3. It calls the application's single `#[init]` function.
4. The application constructs hardware, buses, kernels, backends, devices, and
   ordinary runtime resources.
5. `#[init]` returns `Devices`.
6. `#[devices]` moves those values into static storage.
7. Every field tagged `#[device]` receives `MaybeDevice::start`, which starts
   its generated backend tasks.
8. Every `#[loop_(rate = N)]` function is spawned as an Embassy task.

### Periodic application loops

`rate` is currently interpreted as a delay in milliseconds:

```rust
#[loop_(rate = 100)]
async fn supervision(ctx: Context) {
    // Called, then delayed by 100 ms.
}
```

The application loop owns machine-level scheduling. Declaring an item with
`#[device]` starts its backends, but it does not automatically call
`Device::tick`. The application must decide where and how frequently to tick
the device.

### Device tick

For a generated device, `Device::tick` currently:

1. copies `device.state.custom` into `kernel.state`;
2. runs `kernel.tick()`;
3. copies `kernel.feedback()` back into `device.state.custom`;
4. runs middleware processing.

The duration argument exists in the trait but is not used by the generated
implementation yet.

### Command flow

`Device::execute(command)` forwards the command to the device's middleware.
With no middleware field, the macro inserts `NoMiddleware`; commands then have
no effect.

## 5. Runtime declaration

`#[devices]` parses a struct-like declaration:

```rust
#[devices]
struct Dev {
    #[device]
    top_heater: TopHeater,
    status_led: Output<'static>,
}
```

It generates:

- `Devices`, returned by `#[init]`;
- static storage for every field;
- `StoredContext`, copied into generated loop tasks;
- `ContextView`, exposed to application loop functions;
- `__init_devices__`, which stores resources and starts OREOS devices.

`#[device]` means “this type implements `MaybeDevice`; start it during runtime
initialization.” A plain field is only stored and exposed. `#[shared]` exists,
but its current implementation has unresolved integration and safety concerns;
see the limitations document before using it.

## 6. Macro reference

| Macro | Current responsibility |
|---|---|
| `#[derive(State)]` | Implements `hal::State` and currently emits the hidden `__DeviceState` alias. |
| `#[derive(Config)]` | Implements `hal::Config` and currently emits `__DeviceConfig`. |
| `#[derive(Command)]` | Implements `hal::Command` and currently emits `__DeviceCommand`. |
| `#[derive(GenericBus)]` | Generates the static bus constructor and route-driven `GenericBus` implementation. |
| `#[derive(Kernel)]` | Generates a constructor and the current fixed `Kernel` implementation. |
| `#[derive(Middleware)]` | Implements `Middleware` by delegating to generated `callback_match`. |
| `#[middleware]` | Processes `#[on(Command::Variant)]` handlers; command discovery is currently incomplete. |
| `#[create(Device)]` | Generates the device constructor, `Device`, backend tasks, and `MaybeDevice`. |
| `#[devices]` | Generates runtime storage, `Devices`, and task contexts. |
| `#[app(hal_crate = ...)]` | Generates Embassy main, initialization, and periodic tasks. |

The hidden `__Device*` aliases are current implementation details, not stable
application API. Sema is intended to replace them with semantic type discovery.

## 7. CLI and project model

`ordl` reads and writes `OREOS.toml` in the current directory. The root model is
a single `ControlNode` containing devices. Each device records names for its
state, config, kernel, one bus description, middleware entries, and backends.

Current commands:

```text
ordl new device
ordl new --from DEVICE backend
ordl new --from DEVICE middleware
ordl new runtime
ordl show
ordl show --device DEVICE
ordl generate --device DEVICE
ordl generate --device DEVICE --backend BACKEND
ordl sync
```

`new` updates `OREOS.toml`; it does not necessarily create Rust files.

Full-device generation writes:

```text
src/<device>/
├── mod.rs
├── device.rs
├── kernel.rs
├── <backend>_backend.rs
└── <middleware>_middleware.rs
```

The full generator overwrites all of those generated paths. The backend-only
generator creates only the selected backend and refuses to overwrite an
existing file.

`sync` scans macro metadata under `target/.oreos/tmp`. The metadata graph is
currently incomplete and contains placeholder references, so `sync` must be
reviewed rather than treated as an authoritative round-trip operation.

## 8. Crates

| Crate | Role |
|---|---|
| `oreos-runtime` | Target-side `no_std` traits, wrappers, buses, transports, drivers, and re-exports used by generated code. |
| `oreos-macros` | Compile-time generation for devices and the Embassy runtime. |
| `oreos-cli` | Host-side `ordl` project model and scaffolding. |
| `sema` | Semantic source analysis intended to replace hidden aliases and fragile inter-macro state. |

For the practical workflow and scoping examples, continue with
[Building a system](BUILDING_A_SYSTEM.md).
