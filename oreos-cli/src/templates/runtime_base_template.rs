use crate::templates::format_code;
use quote::quote;

pub fn create_base_runtime() -> String {
    let tokens = quote! {
    #![no_std]
    #![no_main]

    use oreos_runtime::prelude::*;

    oreos::install_defmt_timestamp!();

    #[devices]
    struct Dev {
        led: Output<'static>,
    }


    #[app(hal_crate = your_embassy_crate)]
    mod your_app {
        #[init(config = Config::default())]
        async fn setup(p: Peripherals, _s: Spawner) -> Devices {
            let led = Output::new(p.PA3, Level::Low, Speed::Low);

            Devices {
                led
            }
        }

        #[loop_(rate = 100u64)]
        async fn loop_task(ctx: Context) {
            ctx.led.toggle();
        }
    }
    };

    format_code(tokens)
}
