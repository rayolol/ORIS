# OREOS

A modular embedded robotics operating system framework for Rust-based firmware.

## Crates

- **oreos-runtime**: Core framework with HAL abstractions, kernel, drivers, and motion control for embedded systems
- **oreos-macros**: Procedural macros for defining applications, devices, middleware, and commands
- **oreos-cli**: Command-line tool (`ordl`) for scaffolding and code generation

## Quick Start

Add to your `Cargo.toml`:

Define your application:

```rust
use oreos_runtime::prelude::*;

oreos::install_defmt_timestamp!();

#[devices]
mod devices {
    led: Output<'static>
}

// generates a ctx dependency injection object containing the peripherals and declared devices

#[app(hal_crate = your_embassy_crate)] // only tested on embassy_stm32
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
            //get executed once on task spawn (not stable)
        }
        //2nd loop code
    }
}
```

## Advanced features 

Define a device:


```

    #[create(Device)]
    struct MyDevice {
        #[state] foostate: DeviceState<Mystate> // struct derived under state macro #[derive(State)] and wrapped in DeviceConfig<T: Config>
        #[config] fooconfig: DeviceConfig<MyConfig> // struct derived under state macro #[derive(Config)] and wrapped in DeviceConfig<T: Config>
        #[bus] barbus: MyBus
        #[kernel] barkernel: MyKernel

        #[middleware] mymiddleware: MyMiddleware

        #[backend] mybackend: MyBackend
        #[backend] mybackend2: MyBackend
    }

    more info on [docs/FRAMEWORK.md]
    
```
    if you want to integrate a device in the app runtime. in the #[devices] ctx struct you add:
``` 
    #[devices]
    mod devices {
        #[device]
        dev: MyDevice
    }



    ... in app 

    async fn init(p: Peripherals, s: Spawner) {


        let dev = MyDevice::new(
            /* device init logic */
        );

        dev.start(s);

        Device { dev }
    }
```




## Features

- **Async/await runtime** with Embassy
- **Hardware abstraction** for microcontrollers
- **Command dispatch** framework for device control

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
