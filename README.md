# OREOS

A modular embedded robotics operating system framework for Rust-based ARM firmware.

## Crates

- **oreos-runtime**: Core framework with HAL abstractions, kernel, drivers, and motion control for embedded systems
- **oreos-macros**: Procedural macros for defining applications, devices, middleware, and commands
- **oreos-cli**: Command-line tool (`ordl`) for scaffolding and code generation

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
oreos-runtime = { path = "../oreos/oreos-runtime" }
oreos-macros = { path = "../oreos/oreos-macros" }
```

Define your application:

```rust
use oreos_runtime::prelude::*;

#[devices]
mod devices {
    led: Output<'static>
}

// generates a ctx dependency injection object containing the peripherals and declared devices

#[app]
mod your_app {
    #[init]
    async fn setup(p: Peripherals, s: Spawner) {
        // init code. 
        // devices are created here 

        let led = Output::new(/* embassy's pin init logic*/)

        Devices {
            led
        }
    }

    #[loop_(rate = 1ms)]
    async fn loop(ctx: Context) {
        //loop code 
    }


    #[loop_(rate = 10)]
    async fn loop2(ctx: Context) {
        #[once]
        {
            //clause that get executed once on start up (not stable)
        }
        //2nd loop code
    }
}
```

## Features

- **Async/await runtime** with Embassy
- **Hardware abstraction** for STM32 microcontrollers
- **Motor and stepper drivers** (TMC2209, TMC2160, PWM)
- **Command dispatch** framework for device control

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
