use crate::registry;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub(crate) fn store_command(input: &DeriveInput) {
    let mut input_copy = input.clone();
    input_copy.attrs.clear();
    let tokens = quote! { #input_copy }.to_string();
    registry::store("Command", &input.ident.to_string(), tokens);
}

pub(crate) fn fetch_command(name: &str) -> Option<DeriveInput> {
    registry::fetch("Command", name).and_then(|raw| syn::parse_str(&raw).ok())
}

pub fn derive_command(input: DeriveInput) -> TokenStream {
    let name = &input.ident;

    store_command(&input);

    let expanded = quote! {
        pub type __DeviceCommand = #name;

        impl ::Oreos::hal::Command for #name {}

    };

    TokenStream::from(expanded)
}
