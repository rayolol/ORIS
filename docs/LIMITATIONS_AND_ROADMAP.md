# Limitations and roadmap

OREOS is being exercised in its first official firmware while the architecture
is still changing. This page separates current behavior from intended
direction. It is not a promise that every future item will keep the exact API
shown here.

## 1. Current limitations

### 1.1 Inter-macro type communication uses hidden aliases

The `State`, `Config`, and `Command` derives currently emit module-level aliases:

```rust
pub type __DeviceState = HeaterState;
pub type __DeviceConfig = HeaterConfig;
pub type __DeviceCommand = TopHeaterCommand;
```

Other macros and generated middleware import those names. This imposes an
undocumented rule: one logical device type set per Rust module. A second device
in the same module can collide with the aliases, and moving components between
files breaks the implied relationship.

These aliases are implementation details and will be removed after Sema can
resolve the real types and their module paths.

### 1.2 Sema is connected but not yet the source of macro relationships

`oreos-macros` depends on Sema and can construct a semantic workspace, but the
device-generation path does not yet use it. The command lookup function
currently returns no results, which means middleware `#[on(...)]` handlers are
not a reliable supported path.

The immediate goal is semantic discovery with explicit ambiguity errors—not
more process-global registries or compile-order-dependent communication.

### 1.3 Runtime crate paths are still assumed

Generated code currently uses the fixed dependency name `::oreos`. Firmware
must therefore declare the runtime with that alias:

```toml
oreos = { package = "oreos-runtime", git = "https://github.com/rayolol/ORIS.git" }
```

The macro crate already depends on `proc-macro-crate`, but does not yet use it
consistently to resolve renamed dependencies.

### 1.4 CLI templates and tested macro paths can drift

The templates are not currently compiled as end-to-end fixtures on every
change. For example, runtime and device templates have used inconsistent
`oreos`/`oreos_runtime`/`crate::oreos` paths during development.

A successful build of `oreos-macros` alone does not prove that freshly
generated consumer firmware compiles.

### 1.5 Full generation overwrites application work

```text
ordl generate --device DEVICE
```

rewrites the device, kernel, all backend/middleware files, and `mod.rs`. It has
no merge, dry-run, ownership marker, or conflict preview.

The backend-only generator is safer because it uses create-only file semantics,
but it does not wire the new backend into `device.rs` or `mod.rs`.

### 1.6 Manifest and macro cardinality do not agree

`OREOS.toml` stores middleware as a list, and the generator can emit several
`#[middleware]` fields. `#[create(Device)]` currently retains only the last
middleware field it sees. The supported cardinality should currently be treated
as zero or one.

Similarly, duplicate `#[kernel]`, `#[state]`, `#[config]`, or `#[middleware]`
roles are not rejected cleanly; later fields overwrite earlier selections
during macro parsing.

### 1.7 Backend lifecycle is incomplete

Generated device startup:

- moves each backend into a `StaticCell`;
- spawns one task per backend;
- calls `tick()` every 10 ms.

It does not:

- call `Backend::init`;
- call `Backend::config`;
- use a backend-specific rate;
- route the `tick` return value automatically;
- define shutdown/restart semantics;
- propagate backend errors or conditions into device faults.

The current 10 ms delay is hardcoded.

### 1.8 Device scheduling is application-owned but easy to misunderstand

Putting `#[device]` on a runtime field starts generated backend tasks. It does
not schedule `Device::tick`. The application must call the device tick from a
runtime loop.

The generated `Device::tick` also ignores its duration argument today.

### 1.9 Kernel behavior is fixed by the derive

`#[derive(Kernel)]` generates a basic bus update/write cycle and a trivial
`init`. It does not provide an extension hook for custom control policy inside
the generated trait implementation. Firmware may therefore need helper methods
or future macro changes for real control laws.

### 1.10 Runtime task options are incomplete

- `#[loop_(rate = N)]` treats `N` as a millisecond delay.
- `priority` is parsed but not used when spawning tasks.
- `#[once]` only recognizes attributed local `let` statements and its current
  transformation is not a stable public contract.

### 1.11 Runtime resource sharing needs a safety pass

`#[devices]` uses raw pointers for ordinary and device fields in copied task
contexts. The framework must prove or enforce exclusive access across generated
tasks instead of relying on application discipline.

The `#[shared]` path references shared-cell types that are not yet integrated
as a clean public runtime API. Its current `DerefMut` approach also needs a
careful concurrency and critical-section review.

Until that work is complete, avoid sharing one mutable resource across several
generated loops.

### 1.12 Hardware configuration is manual

The framework does not currently know:

- which board or exact MCU package is used;
- which pins exist or are reserved;
- which alternate-function mappings are legal;
- which peripheral instance owns a pin;
- whether two backends claim the same resource;
- clock, DMA, interrupt, or timer-channel constraints.

These choices live in application initialization and are not validated against
`OREOS.toml`.

### 1.13 Project initialization is not complete

`ordl new runtime` only scaffolds `src/main.rs`. A flawless project initializer
would also need to coordinate:

- `Cargo.toml` and compatible dependency versions/features;
- Rust toolchain and target;
- `.cargo/config.toml`;
- linker script and `memory.x`;
- board/MCU selection;
- panic/logging setup;
- build, flash, and debug configuration.

The macro crate currently uses a nightly-only proc-macro span feature, so a
compatible nightly toolchain is required.

### 1.14 Metadata sync is provisional

Macros write JSON metadata to `target/.oreos/tmp` and `ordl sync` scans it, but
device references are still emitted as placeholder values such as `unbounded`.
The sync engine can register unresolved components and block implicit deletion,
but it cannot yet guarantee a lossless TOML ↔ Rust round trip.

Stale metadata, incremental compilation, duplicate unresolved entries, and
target-directory selection still need explicit handling.

### 1.15 Diagnostics and documentation are incomplete

Many malformed declarations currently fail at the macro invocation with a
generic Rust error. The macros need errors that name:

- the missing or duplicated role;
- the device/component involved;
- the expected declaration;
- the source locations of conflicting items;
- a concrete repair.

Generated code also needs compile-tested documentation examples and supported
board/toolchain matrices.

## 2. Near-term plan

The order matters: stabilize semantic relationships and diagnostics before
building a richer editor on top of an unstable model.

### Phase 1: establish a tested current baseline

- Add a small consumer fixture that runs `ordl` and compiles the generated
  firmware-facing source.
- Test runtime-only, device-with-no-middleware, device-with-backend, and
  device-with-middleware cases.
- Test dependency renaming and runtime re-exports.
- Keep the first real firmware as a practical integration test, but do not make
  it the only test.
- Turn recent failures into regression tests: runtime path casing,
  `static_cell` resolution, missing command association, and generic middleware
  inference.

### Phase 2: make Sema the semantic bridge

Replace hidden aliases and ad hoc inter-macro state with source analysis.

The first focused Sema slice should:

1. identify the module containing `#[create(Device)]`;
2. discover the associated `State`, `Config`, and `Command` declarations;
3. resolve canonical module-qualified paths;
4. validate exactly one unambiguous association;
5. let middleware inspect the real command enum and variants;
6. emit actionable errors for missing or ambiguous relationships;
7. remove `__DeviceState`, `__DeviceConfig`, and `__DeviceCommand`;
8. update CLI templates so they no longer import hidden aliases.

Sema should provide read-only semantic facts to macros. It should not recreate a
mutable global registry whose result depends on expansion order.

If proximity alone becomes ambiguous, add explicit relationships to generated
source or the project model rather than guessing.

### Phase 3: harden generation

- Make component generation create-only by default.
- Add standalone generation for backend, middleware, kernel, and device shell.
- Add `--dry-run` and a visible file/action plan.
- Refuse destructive regeneration unless the user explicitly opts in.
- Decide which files are generator-owned and which are application-owned.
- Update `mod.rs` and device fields through syntax-aware edits rather than full
  replacement.
- Add validation before writing any file.
- Make CLI help and error messages use the exact accepted command grammar.

The goal is safe incremental work, not perpetual full regeneration.

### Phase 4: complete device lifecycle behavior

- [ ] Make every backend constructor establish its hardware-safe state before
  returning; for example, a heater backend must drive its SSR OFF immediately.
- [ ] Give every backend instance `&'static` references to the bus-owned state
  and configuration channels it consumes; do not create a second authoritative
  state/config copy inside the backend.
- [ ] Keep `MaybeDevice::start` synchronous: move each already-wired backend
  into its `StaticCell` and spawn its generated task.
- [ ] At the beginning of the generated backend task, obtain the initial
  configuration snapshot through the backend's configuration channel and call
  `backend.init(init_config).await` exactly once before entering the repeating
  `tick()` loop. If the trait changes to parameterless `init`, require that
  method to perform the same initial channel read.
- [ ] If backend initialization fails, publish an initialization fault, keep the
  hardware safe, and return without ever calling `tick()`.
- [ ] After successful initialization, call `tick()` at the configured backend
  rate until shutdown or fault policy stops the task.
- [ ] Version configuration snapshots and define when later calls to
  `Backend::config` run as newer revisions arrive, without repeating
  initialization.
- [ ] Preserve `State` and `Config` as the only backend data-flow contracts; do
  not introduce separate command/feedback transport types.
- [ ] Add an atomic field-update/merge operation to the lane abstraction. The
  bus and a backend may update different fields of one state, so generated
  read-modify-write sequences must not lose updates and `FastLane` must safely
  coordinate multiple writers.
- [ ] Make backend scheduling configurable.
- [ ] Route backend outputs and health into the bus/device state deliberately.
- [ ] Define device/kernel initialization ordering.
- [ ] Use the `dt` supplied to `Device::tick`.
- [ ] Make the generated kernel/device path invoke middleware without placing
  application-specific behavior in the kernel.
- [ ] Validate role cardinality and backend/middleware relationships.

### Phase 5: improve runtime ownership

- Replace unchecked task-context aliasing with an ownership model the compiler
  or generated synchronization can enforce.
- Finish and expose shared resources through the runtime crate.
- Define which task owns each non-shared resource.
- Apply or remove the task priority option.
- Replace `#[once]` with a clear, testable initialization mechanism.

## 3. Two-stage hardware configuration

The planned hardware model has two separate layers because they answer different
questions.

### Stage A: board profile

The board profile describes capability:

- MCU family and exact part/package;
- available pins;
- alternate functions;
- timers, channels, ADCs, buses, DMA, and interrupts;
- clock assumptions;
- reserved/debug pins;
- memory/linker information;
- supported Embassy HAL feature set.

This layer says what the board can do. It should be reusable by several
firmware projects.

### Stage B: system hardware binding

The system/node configuration assigns board resources to actual component
instances:

```text
TopHeater.HeaterControl.output -> TIMx channel y / pin PAz
TopHeater.TemperatureSense.spi -> SPIx
TopHeater.TemperatureSense.cs  -> pin PBz
```

Backends declare logical access requirements, not concrete pins or MCU
peripheral types. The midlayer binding resolves those requirements and
generates the access value injected into each backend. An access value may
contain direct IO handles, transport clients, or both:

```text
backend logical requirements
        ↓
midlayer pin/peripheral assignment
        ├── direct GPIO/timer/ADC capability
        ├── target-aware TransportClient
        └── combined generated access bundle
        ↓
backend constructor
```

For a shared bus, one `TransportServer` owns and serializes the physical
peripheral. Generated clients carry or select the intended slave target. This
keeps the physical bus accessible through controlled high-level clients without
giving several backends mutable ownership of the raw peripheral.

The backend template should remain hardware-agnostic. ORDL/Sema generates the
concrete access bundle only after the board profile and node binding are known.
State and config remain the runtime data-flow contracts; hardware access is an
injected capability, not another state channel.

This layer should validate:

- exclusive pin/peripheral ownership;
- alternate-function compatibility;
- required capabilities for each backend;
- timer/DMA/interrupt conflicts;
- deliberate sharing;
- transport target validity and response correlation;
- late changes that would invalidate another device.

The result should feed typed application wiring or generated constructors while
leaving runtime policy in application code.

The distinction is essential:

- the board profile is hardware capability;
- the binding is this machine's allocation;
- the generated access bundle is how a backend reaches that allocation;
- `DeviceConfig` is device behavior/policy;
- runtime initialization constructs the concrete values.

## 4. System and ORIS evolution

After node/device semantics are stable, the model can grow toward the wider
ORIS system:

- multi-node manifests;
- stable node and device identities;
- transport endpoints and message schemas;
- cross-device and cross-node connections;
- capability discovery;
- deployment and compatibility checks;
- machine-level observability.

OREOS should remain the embedded implementation layer. It should not absorb all
application logic or turn the ORIS system description into hidden generated
runtime policy.

## 5. Future user interface

A graphical app or VS Code extension is a sensible future interface once
`OREOS.toml` and the semantic model become too dense to inspect comfortably.
It should be another client of the same validated model—not a second source of
truth.

Useful future views include:

- system → node → device hierarchy;
- device component graph;
- board pin/peripheral allocation;
- ownership and conflict diagnostics;
- generated-file previews;
- safe add/remove/refactor operations;
- direct navigation from model elements to Rust source.

The CLI should remain scriptable and testable underneath that interface.

## 6. What “flawless init” should eventually mean

A successful initialization command should produce a project that:

- has a selected board and Rust target;
- contains compatible dependencies and features;
- has linker/memory and Cargo target configuration;
- builds before application behavior is added;
- has a valid runtime skeleton;
- explains the next command;
- never silently overwrites application work;
- reports unsupported board/toolchain combinations clearly;
- can add a device incrementally without regenerating unrelated code.

That end state should be reached through the phases above rather than by
building the full editor, system graph, and hardware model at once.
