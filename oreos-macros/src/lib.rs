#![feature(proc_macro_span)]

mod app;
mod command;
mod derive;
mod devices;
mod metadata;
mod middleware;
mod registry;
mod transport;

/*
    TODO: make a derive macro for the Backend type
    TODO: Make better error handling for macros internal
    TODO: refactor this file, put each macro in its own file, and put shared code in a utils module
    TODO: use darling for future macros, it makes parsing attributes much easier
*/

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemImpl, ItemStruct, parse_macro_input, spanned::Spanned};

fn save_macro_metadata(name: &str, payload: crate::metadata::ComponentPayload) {
    let span = proc_macro::Span::call_site();
    let source = crate::metadata::LineInfo {
        file: span.file(),
        line: span.line() as u32,
    };
    let meta = crate::metadata::Metadata::new(name.to_string(), source, payload);
    meta.save().expect("Failed to save metadata");
}

pub(crate) struct AppArgs {
    pub hal_crate: syn::Ident,
}

impl syn::parse::Parse for AppArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _: syn::Ident = input.parse()?;
        let _: syn::Token![=] = input.parse()?;
        let hal_crate: syn::Ident = input.parse()?;
        Ok(AppArgs { hal_crate })
    }
}

pub(crate) struct DeviceList {
    pub name: syn::Ident,
    pub items: Vec<DeviceItem>,
}

pub(crate) struct DeviceItem {
    pub attrs: Vec<syn::Attribute>,
    pub name: syn::Ident,
    pub shared: bool,
    pub ty: syn::Type,
}

impl syn::parse::Parse for DeviceList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Token![struct]>()?;
        let name: syn::Ident = input.parse()?;
        let content;
        syn::braced!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(content.parse::<DeviceItem>()?);
        }
        Ok(DeviceList { name, items })
    }
}

impl syn::parse::Parse for DeviceItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let shared = attrs.iter().any(|a| a.path().is_ident("shared"));
        let name: syn::Ident = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let ty: syn::Type = input.parse()?;

        let _ = input.parse::<syn::Token![,]>().ok();
        Ok(DeviceItem {
            name,
            ty,
            shared,
            attrs,
        })
    }
}

#[proc_macro_attribute]
pub fn app(attr: TokenStream, item: TokenStream) -> TokenStream {
    app::app(attr, item)
}

#[proc_macro_attribute]
pub fn devices(attr: TokenStream, item: TokenStream) -> TokenStream {
    devices::devices(attr, item)
}

#[proc_macro]
pub fn make_transport(item: TokenStream) -> TokenStream {
    transport::make_transport(item)
}

fn convert_struct_to_derive_input(item_struct: ItemStruct) -> syn::Result<DeriveInput> {
    // 1. Convert the ItemStruct back into a TokenStream
    let tokens = quote::quote! { #item_struct };

    // 2. Parse that TokenStream straight into a DeriveInput
    let derive_input: DeriveInput = syn::parse2(tokens)?;

    Ok(derive_input)
}

#[proc_macro_attribute]
pub fn create(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemStruct = parse_macro_input!(item as ItemStruct);

    let derive_input = match convert_struct_to_derive_input(input) {
        Ok(d) => d,
        Err(e) => return e.into_compile_error().into(),
    };

    save_macro_metadata(
        &derive_input.ident.to_string(),
        crate::metadata::ComponentPayload::Device {
            refs: crate::metadata::DeviceRefs {
                //TODO: parse these from attributes
                state: "unbounded".to_string(),
                config: "unbounded".to_string(),
                kernel_bus: "unbounded".to_string(),
                backends: vec![],
                middleware: None,
            },
        },
    );

    match derive::create_device(derive_input) {
        Ok(t) => t.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(GenericBus, attributes(route, state))]
pub fn make_bus(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);

    save_macro_metadata(
        &input.ident.to_string(),
        crate::metadata::ComponentPayload::Bus {
            lanes: vec![
                //TODO: parse these from attributes
                crate::metadata::BusLaneInfo {
                    name: "main".to_string(),
                    lane_type: "sync".to_string(),
                    bound: "unbounded".to_string(),
                },
            ],
        },
    );

    match derive::impl_bus(input) {
        Ok(t) => t.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(Kernel, attributes(state, config, bus))]
pub fn make_kernel(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);

    save_macro_metadata(
        &input.ident.to_string(),
        crate::metadata::ComponentPayload::Kernel {
            bus: "unbounded".to_string(),
        },
    );

    match derive::impl_kernel(input) {
        Ok(t) => t.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(Middleware)]
pub fn make_middleware(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);

    save_macro_metadata(
        &input.ident.to_string(),
        crate::metadata::ComponentPayload::Middleware,
    );

    match derive::derive_middleware(input) {
        Ok(t) => t.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn middleware(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemImpl = parse_macro_input!(item as ItemImpl);

    match middleware::middleware(input) {
        Ok(t) => t.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(State)]
pub fn make_state(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);

    save_macro_metadata(
        &input.ident.to_string(),
        crate::metadata::ComponentPayload::State,
    );

    derive::derive_state(input).into()
}

#[proc_macro_derive(Config)]
pub fn make_config(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);

    save_macro_metadata(
        &input.ident.to_string(),
        crate::metadata::ComponentPayload::Config,
    );

    derive::derive_config(input).into()
}

#[proc_macro_derive(Command)]
pub fn make_command(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item as DeriveInput);

    save_macro_metadata(
        &input.ident.to_string(),
        crate::metadata::ComponentPayload::Command,
    );

    command::derive_command(input).into()
}
