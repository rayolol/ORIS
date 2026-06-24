use quote::{quote, format_ident};


pub fn kernel_template(name: &str, state: &str, config: &str, bus: &str) -> String {
    let name = format_ident!("{}", name);
    let state = format_ident!("{}", state);
    let config = format_ident!("{}", config);
    let bus = format_ident!("{}", bus);


    let tokens = quote! {

        use oreos::prelude::*;

        #[derive(GenericBus)]
        struct #bus {
            #[state]
            state: #state,
            //TODO use the lanes and route them
        }

        #[derive(Kernel)]
        struct #name {
            #[state]
            state: #state,
            #[config]
            config: #config,
            #[bus]
            bus: &'static #bus
        }

    };

    prettyplease::unparse(&syn::parse2(tokens).unwrap())
}


pub fn middleware_template(name: &str) -> String {
    let name = format_ident!("{}", name);
    let tokens = quote! {

        use oreos::prelude::*;

        #[derive(Middleware)]
        struct #name {
            // TODO: Implement middleware logic
        }

        #[middleware]
        impl #name {
            // TODO: Implement middleware callback methods
        }
    };
    prettyplease::unparse(&syn::parse2(tokens).unwrap())
}

pub fn backend_template(name: &str) -> String {
    let name = format_ident!("{}", name);
    let tokens = quote! {

        use oreos::prelude::*;


        struct #name {
            // TODO
        }

        impl Backend for #name {
            pub fn new() -> Self {
                Self {}
            }

            pub async fn init(&mut self) {
                todo!("Initialize backend")
            }

            pub async fn tick(&mut self) {
                todo!("Backend tick")
            }
        }
    };
    prettyplease::unparse(&syn::parse2(tokens).unwrap())
}

pub fn device_template(
    name: &str,
    state: &str,
    kernel: &str,
    config: &str,
    middlewares: &[&str],
    backends: &[&str],
) -> String {
    let name = format_ident!("{}", name);
    let state = format_ident!("{}", state);
    let kernel = format_ident!("{}", kernel);
    let config = format_ident!("{}", config);

    let middlewares: Vec<_> = middlewares.iter().map(|m| format_ident!("{}", m)).collect();
    let backends: Vec<_> = backends.iter().map(|b| format_ident!("{}", b)).collect();

    let tokens = quote! {
        use oreos::prelude::*;
        // import the components location

        pub struct #state {
            // TODO
        }

        pub struct #config {
            // TODO
        }

        #[create(Device)]
        pub struct #name {
            #[kernel] kernel: #kernel,
            #[state] state: DeviceState<#state>,
            #[config] config: DeviceConfig<#config>,
            #(#[middleware] middleware: #middlewares),*
            #(#[backends] backend: #backends),*
        }
    };

    prettyplease::unparse(&syn::parse2(tokens).unwrap())
}