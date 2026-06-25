use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::casing::{snake_with_suffix, to_pascal_case, to_snake_case};

/// Pretty-prints `tokens` and inserts blank lines between top-level items, since
/// prettyplease formats a single parsed `syn::File` with no item spacing of its own.
fn format_code(tokens: TokenStream) -> String {
    let file = syn::parse2(tokens).expect("generated tokens must parse as a valid Rust file");
    let pretty = prettyplease::unparse(&file);
    insert_blank_lines_between_items(&pretty)
}

/// Inserts a blank line before every top-level item (or its leading attribute),
/// without separating stacked attributes from the item they annotate.
fn insert_blank_lines_between_items(code: &str) -> String {
    let mut out = String::with_capacity(code.len() + 64);
    let mut depth: i32 = 0;
    let mut prev_was_blank = true;
    let mut prev_was_attribute = false;
    let mut prev_opened_block = false;

    for line in code.lines() {
        let trimmed = line.trim_start();
        // Top-level items (structs, impls, derives...) get spaced on sight. One
        // level deeper, only function definitions get spaced — that covers methods
        // inside an `impl` block without inserting blank lines between struct fields.
        let is_top_level_item = depth == 0
            && (trimmed.starts_with("pub ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("#["));
        let is_nested_fn = depth == 1
            && (trimmed.starts_with("fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub async fn "));
        let starts_item =
            (is_top_level_item || is_nested_fn) && !prev_was_attribute && !prev_opened_block;

        if starts_item && !prev_was_blank {
            out.push('\n');
        }

        out.push_str(line);
        out.push('\n');

        prev_was_blank = trimmed.is_empty();
        prev_was_attribute = trimmed.starts_with("#[");
        prev_opened_block = trimmed.ends_with('{');
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
    }

    out
}

pub fn kernel_template(name: &str, state: &str, config: &str, bus: &str, device: &str) -> String {
    let name = format_ident!("{}", to_pascal_case(name));
    let state = format_ident!("{}", to_pascal_case(state));
    let config = format_ident!("{}", to_pascal_case(config));
    let bus = format_ident!("{}Bus", to_pascal_case(bus));
    let device_module = format_ident!("{}", to_snake_case(device));

    let tokens = quote! {
        use crate::oreos::prelude::*;
        use crate::#device_module::device::{#state, #config};

        #[derive(GenericBus)]
        pub struct #bus {
            #[state]
            pub state: #state,
            pub estop: EstopFlag,
            //TODO use the lanes and route them
        }

        #[derive(Kernel)]
        pub struct #name {
            #[state]
            pub state: #state,
            #[config]
            pub config: #config,
            #[bus]
            pub bus: &'static #bus
        }
    };

    format_code(tokens)
}

pub fn middleware_template(name: &str, device: &str) -> String {
    let name = format_ident!("{}", to_pascal_case(name));
    let device_module = format_ident!("{}", to_snake_case(device));
    let tokens = quote! {
        use crate::oreos::prelude::*;
        use crate::#device_module::device::{__DeviceCommand, __DeviceState, __DeviceConfig};

        #[derive(Middleware)]
        pub struct #name {
            // TODO: Implement middleware logic
        }

        #[middleware]
        impl #name {
            // TODO: Implement middleware callback methods
        }
    };
    format_code(tokens)
}

pub fn backend_template(name: &str) -> String {
    let pascal = to_pascal_case(name);
    let name = format_ident!("{}", pascal);
    let backend_state = format_ident!("{}State", pascal);
    let condition = format_ident!("{}Condition", pascal);

    let tokens = quote! {
        use crate::oreos::prelude::*;

        pub struct #backend_state {
            //TODO
        }

        pub struct #condition;

        impl Condition for #condition {
            fn classify(&self, _ctx: &DeviceState<impl StateTrait>) -> Fault {
                todo!("classify backend condition")
            }
        }

        pub struct #name {
            // TODO
        }

        impl Backend for #name {
            type Output = ();
            type Condition = #condition;
            type Config = ();
            type Error = ();

            async fn init(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
                todo!("Initialize backend")
            }

            async fn tick(&mut self) -> Self::Output {
                todo!("Backend tick")
            }

            async fn config(&mut self, _config: Self::Config) -> Result<(), Self::Error> {
                todo!("Backend config")
            }
        }
    };
    format_code(tokens)
}

pub fn device_template(
    name: &str,
    state: &str,
    kernel: &str,
    config: &str,
    middlewares: &[&str],
    backends: &[&str],
) -> String {
    let module = format_ident!("{}", to_snake_case(name));
    let pascal_name = to_pascal_case(name);
    let name = format_ident!("{}", pascal_name);
    let state = format_ident!("{}", to_pascal_case(state));
    let kernel = format_ident!("{}", to_pascal_case(kernel));
    let config = format_ident!("{}", to_pascal_case(config));
    let command = format_ident!("{}Command", pascal_name);

    // Each extra field carries its own attribute, so the whole group can be joined
    // by a single `,`-separated repetition without worrying about a missing comma
    // between the middleware fields and the backend fields.
    let extra_fields: Vec<TokenStream> = middlewares
        .iter()
        .map(|m| {
            let field = format_ident!("{}", to_snake_case(m));
            let ty = format_ident!("{}", to_pascal_case(m));
            quote! { #[middleware] #field: #ty }
        })
        .chain(backends.iter().map(|b| {
            let field = format_ident!("{}", to_snake_case(b));
            let ty = format_ident!("{}", to_pascal_case(b));
            quote! { #[backend] #field: #ty }
        }))
        .collect();

    // Each middleware/backend lives in its own sibling module (the file `ordl
    // generate` writes it to), so `#[create(Device)]` needs an explicit import
    // of its type alongside the kernel's.
    let component_imports: Vec<TokenStream> = middlewares
        .iter()
        .map(|m| {
            let mod_name = format_ident!("{}", snake_with_suffix(m, "middleware"));
            let ty = format_ident!("{}", to_pascal_case(m));
            quote! { use crate::#module::#mod_name::#ty; }
        })
        .chain(backends.iter().map(|b| {
            let mod_name = format_ident!("{}", snake_with_suffix(b, "backend"));
            let ty = format_ident!("{}", to_pascal_case(b));
            quote! { use crate::#module::#mod_name::#ty; }
        }))
        .collect();

    let tokens = quote! {
        use crate::oreos::prelude::*;
        use crate::#module::kernel::#kernel;
        #(#component_imports)*

        #[derive(State, Clone, Copy)]
        pub struct #state {
            // TODO
        }

        #[derive(Config)]
        pub struct #config {
            // TODO
        }

        #[derive(Command)]
        pub enum #command {
            // TODO: add command variants, then handle them with #[on(...)]
            // in a #[middleware] impl
        }

        #[create(Device)]
        pub struct #name {
            #[kernel] kernel: #kernel,
            #[state] state: DeviceState<#state>,
            #[config] config: DeviceConfig<#config>,
            #(#extra_fields),*
        }
    };

    format_code(tokens)
}
