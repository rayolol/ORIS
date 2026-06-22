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
use oreos_macros::app;

#[app]
pub struct MyApp {
    // Define your devices and state here
}

#[tokio::main]
async fn main() {
    let mut app = MyApp::new();
    app.run().await;
}
```

## Features

- **Async/await runtime** with Embassy
- **Hardware abstraction** for STM32F1xx microcontrollers
- **Motor and stepper drivers** (TMC2209, TMC2160, PWM)
- **Middleware system** for cross-cutting concerns
- **Command dispatch** framework for device control

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
