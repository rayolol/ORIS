use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::workspace;

pub(crate) fn find_commands(name: &str) -> Option<Vec<DeriveInput>> {
    // let ws = workspace();
    // let enums = ws.enums().named(name).collect();

    None
}

pub fn derive_command(input: DeriveInput) -> TokenStream {
    let name = &input.ident;

    // store_command(&input);

    let expanded = quote! {

        impl ::oreos::hal::Command for #name {}

    };

    TokenStream::from(expanded)
}
