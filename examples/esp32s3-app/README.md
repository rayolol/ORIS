# OREOS application on ESP32-S3

This example uses OREOS's existing application model with `esp-hal`:

```rust
#[app(hal_crate = esp_hal, entry = esp_rtos::main)]
mod app {
    #[init(config = esp_hal::Config::default())]
    async fn init(p: esp_hal::peripherals::Peripherals, _: Spawner) -> Devices {
        let timers = TimerGroup::new(p.TIMG0);
        esp_rtos::start(timers.timer0, p.FROM_CPU_INTR0);
        // Construct your drivers/devices using the remaining peripherals.
        // Return Devices here.
    }
}
```

OREOS calls `esp_hal::init(config)`, passes the ESP peripherals into your init,
stores the returned devices, and starts your `#[loop_]` tasks. The application
selects the entry macro with `entry`; existing applications that omit it keep
`embassy_executor::main`.

The OREOS dependency uses `default-features = false` to disable its existing STM32
Cargo configuration. `esp-rtos` supplies the ESP executor and Embassy time driver;
the application's init starts it using TIMG0 timer 0 and FROM_CPU_INTR0.

## Build

Install the [Espressif Rust toolchain](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
with Rust 1.95 or newer:

```sh
espup install --targets esp32s3
. "$HOME/export-esp.sh"
cd examples/esp32s3-app
cargo build --release
```

Verified with the ESP Rust 1.95.0 toolchain, `esp-hal` 1.2.2, and `esp-rtos` 0.4.0.
The linked release binary targets `xtensa-esp32s3-none-elf`. Hardware execution
has not been tested.

The example prints a startup message and one update per second over the default
`esp-println` transport. To flash and monitor it with `espflash`:

```sh
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/oreos-esp32s3-app-example
```

## Adding a CYD interface

This is an ESP32-S3 framework bring-up example. Display and touch drivers are not
initialized yet: ESP32-S3 CYD variants have different display controllers and
pinouts. Once the exact board model is known, construct those drivers in `init`
and add their resource types to `AppDevices`. The same OREOS `#[devices]`,
`Context`, and `#[loop_]` API then exposes them to the interface loop.

Initialization follows the published [esp-rtos 0.4.0 setup](https://docs.rs/esp-rtos/0.4.0/esp_rtos/).
