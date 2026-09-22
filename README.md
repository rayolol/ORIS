# OREOS

OREOS (**Open Robotics Embedded Operating System**) is the embedded firmware
layer of ORIS (**Open Robotics Interface Standard**). It provides Rust and
Embassy building blocks for describing a control node as a runtime containing
typed, active devices.

OREOS is currently experimental. The device/runtime path compiles and is being
validated in the first real firmware project, but the CLI, macro communication,
hardware configuration, diagnostics, and public API are still evolving.

## Repository layout

- `oreos-runtime`: `no_std` HAL traits, state/config wrappers, buses, transports,
  drivers, and runtime support.
- `oreos-macros`: device, kernel, bus, application, and runtime code generation.
- `oreos-cli`: the `ordl` host tool and the `OREOS.toml` project model.
- `docs`: current behavior, practical workflows, limitations, and roadmap.

## Start here

- [Framework model](docs/FRAMEWORK.md): what a node, device, kernel, bus,
  backend, and middleware mean today.
- [Building a system](docs/BUILDING_A_SYSTEM.md): scoping rules and the complete
  node/device workflow.
- [Limitations and roadmap](docs/LIMITATIONS_AND_ROADMAP.md): what is incomplete,
  what is unsafe to assume, and the planned Sema and hardware-configuration work.

## Current CLI path

From an OREOS firmware project containing `OREOS.toml`:

```text
ordl new device
ordl new --from TopHeater backend
ordl generate --device TopHeater
ordl generate --device TopHeater --backend HeaterControl
```

The full-device generator currently overwrites its generated files. The
backend-only form is create-only and refuses to overwrite an existing backend.
Read the workflow guide before regenerating edited device code.

## License

Licensed under either Apache-2.0 or MIT, at your option.
