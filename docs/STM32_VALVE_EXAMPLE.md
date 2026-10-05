# STM32F103C8 valve device

The [complete source](../examples/stm32f103-valve/) is a small, hand-completed
OREOS device for an STM32F103C8. It is an example of what to build **after**
using the CLI scaffold: the CLI does not choose pins, write bus routes, or
implement the backend. This example needs no ORDL hardware schema.

The example was checked and linked for `thumbv7m-none-eabi` against the current
OREOS checkout. It has not been flashed or measured on a physical valve.

## Build

Install a nightly Rust toolchain and its `thumbv7m-none-eabi` target, then run:

```sh
cd examples/stm32f103-valve
cargo +nightly build --release --target thumbv7m-none-eabi
```

The example contains its own [Cargo manifest](../examples/stm32f103-valve/Cargo.toml),
[linker configuration](../examples/stm32f103-valve/.cargo/config.toml),
and [memory map](../examples/stm32f103-valve/memory.x). The manifest uses a
relative path to `oreos-runtime`, so build it from this repository checkout.

## Board wiring used by the example

| Resource | STM32 handle | Role |
|---|---|---|
| Digital output | PB0 | Requests the external valve driver on/off |
| Digital input | PB1 | Reports whether the valve is open |
| ADC input | PA2 through ADC1 | Publishes a raw analog reading |
| PWM output | PA1 through TIM2 channel 2 | Sets 30% duty when opening, 0% when closing |

PB0 is a logic output for an external valve driver; the example does not
provide a power stage or an electrical schematic. Change the pins and driver
behavior in `make_valve_with_stm32_pins` for your board.

## Follow the data

The [device definition](../examples/stm32f103-valve/src/lib.rs) has two state
layers:

- `ValveState` is device-level intent and feedback. It lives in
  `DeviceState<ValveState>` and in the kernel's state.
- `ValveData` is the backend lane payload. It is the data exchanged with
  `ValveBackend`; the bus translates fields between these types.

`ValveBus` routes `requested_open` from device state into `ValveData` with
`=>`. It routes `actual_open` and `analog_raw` from `ValveData` into device
state with `<=`. The backend reads its lane, drives the output and PWM channel,
samples the input and ADC, then writes feedback to the same lane.

`ValveAccess` groups ordinary Embassy handles with `#[io(kind = "digital")]`,
`#[io(kind = "analog")]`, and `#[io(kind = "pwm")]`. Its derived constructor
receives handles made in `make_valve_with_stm32_pins`. The `kind` attribute
categorizes each handle; it does not create or assign a pin.

The [runtime](../examples/stm32f103-valve/src/main.rs) constructs the device
from the STM32 peripherals, starts the backend task through `#[devices]`, and
calls `ValveDevice::tick` every 10 ms. It alternates `Open` and `Close` commands
every two seconds. `ValvePolicy` handles those commands through a manual
`Middleware` implementation because cross-macro `#[on(...)]` resolution is not
available yet. `ValveCondition` currently reports `Healthy` unconditionally;
real fault detection must replace that example policy.

The `TIME_READY` flag in `main.rs` belongs only to defmt timestamps. Early log
messages use timestamp zero until Embassy time is initialized in `#[init]`.
It is not device or backend state.

## Applying this after CLI generation

Use the CLI to create the files and names, then copy the *relationships* shown
in the example into your device:

1. Add a backend lane and explicit `#[route(...)]` mappings to the bus.
2. Fill the device and backend payloads, and use `DeviceState<T>` and
   `DeviceConfig<T>` on the device.
3. Define `IoAccess` fields with the Embassy handle types you actually own.
4. Implement the backend's `init`, `tick`, and `config` methods and its condition.
5. Implement middleware command handling and assemble the bus, backend, kernel,
   and device from board resources in application code.

The current CLI device scaffold uses `#[backend(rate: "10")]`; the current
device macro expects `#[backend(tick_rate: "10")]`, as shown in this example.
Adjust that attribute while completing a generated scaffold.
