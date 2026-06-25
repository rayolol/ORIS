use std::fmt::format;

use proc_macro_error::{abort, emit_error};
use syn::{DeriveInput, Ident, ItemImpl, spanned::Spanned};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Fields;
use convert_case::{Case, Casing};

use crate::{command};


fn generate_struct(variant_name: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(named) => {
            let fields_decls: Vec<_> = named.named.iter().map(|f| {
                let ident = &f.ident;
                let ty = &f.ty;
                quote! {
                    pub #ident: #ty
                }
            }).collect();

            quote! {
                pub struct #variant_name {
                    #(#fields_decls),*
                }
            }
        },

        _ => quote! {}
    }
}




fn extract_variant(input: &DeriveInput, variant_name: &Ident, handler_name: &Ident) -> TokenStream {
    if let syn::Data::Enum(ref data_enum) = input.data {
        let enum_name = &input.ident;

        let variant = match data_enum.variants.iter().find(|v| v.ident == *variant_name) {
            Some(v) => v,
            None => {
                abort! (
                    variant_name.span(),
                    "variant not found in enum"
                )
            }
        };

        match &variant.fields {
            Fields::Unit => {
                quote! {
                    #enum_name::#variant_name => Self::#handler_name(state, config)
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_names: Vec<_> = (0..field_count)
                    .map(|i| format_ident!("field{}", i))
                    .collect();

                let pattern = if field_count == 1 {
                    let f = &field_names[0];
                    quote! { #enum_name::#variant_name(#f) }
                } else {
                    quote! { #enum_name::#variant_name(#(#field_names),*) }
                };

                quote! {
                    #pattern => Self::#handler_name(state, config, #(#field_names),*)
                }
            }
            Fields::Named(named) => {
                let struct_name = format_ident!("{}", variant_name.to_string().to_case(Case::Pascal));
                let field_names: Vec<_> = named.named.iter()
                    .filter_map(|f| f.ident.as_ref())
                    .collect();
      
                quote! {
                    #enum_name::#variant_name { #(#field_names),* } => {
                        Self::#handler_name(state, config, #struct_name { #(#field_names),* })
                    }
                }
            }
        }
    } else {
        abort!(
            input.span(),
            "input is not an enum"
        )
    }
}

struct OnAttr {
    variant: syn::Path
}

impl syn::parse::Parse for OnAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let variant: syn::Path = input.parse()?;
        return Ok (OnAttr { variant })
    }
}




pub fn middleware(mut input: ItemImpl) -> syn::Result<TokenStream> {

    let mut on_statements = Vec::new();

    let mut struct_definitions = Vec::new();

    // First pass: collect on statements and remove #[on(...)] attributes
    for item in &mut input.items {
        if let syn::ImplItem::Fn(function) = item {
            let mut on_attrs = Vec::new();

            function.attrs.retain(|attr| {
                if attr.path().is_ident("on") {
                    on_attrs.push(attr.clone());
                    false
                } else {
                    true
                }
            });

            for attr in on_attrs {
                let func_name = &function.sig.ident;
                let command_path: OnAttr = attr.parse_args()?;
                let command_path_token = command_path.variant;


                let command_enum = &command_path_token.segments.first().unwrap_or_else(|| {
                    abort!(
                        attr.span(),
                        "#[on(...)] expect format like #[on(MyCommand::Variant)]"
                    )
                }).ident;

                let variant_name = &command_path_token.segments.last().unwrap_or_else(|| {
                    abort!(
                        attr.span(),
                        "#[on(...)] expect format like #[on(MyCommand::Variant)]"
                    )
                }).ident;

                let Some(commands) = command::fetch_command(&command_enum.to_string()) else {
                    emit_error!(
                        attr.span(),
                        "command not {}::{} found", command_enum, variant_name
                    );
                    return Ok(quote! {})
                };


                if let syn::Data::Enum(ref data_enum) = commands.data {
                    let Some(variant) = data_enum.variants.iter().find(|v| v.ident == *variant_name) else {
                        emit_error!(
                        attr.span(),
                        "command not {}::{} found", command_enum, variant_name
                    );
                    return Ok(quote! {})
                    };

       
                    
                    if matches!(variant.fields, Fields::Named(_)) {
                        struct_definitions.push(generate_struct(&variant_name, &variant.fields)); 
                    }
                }

                let arm = extract_variant(&commands, &variant_name, func_name);

                on_statements.push(arm)
                
            }
        }
    }

    let impl_self_ty = &input.self_ty;
    let impl_items = &input.items;

    let expanded = quote! {
        #(#struct_definitions)*

        impl #impl_self_ty {
            #(#impl_items)*

            fn callback_match(&mut self, cmd: __DeviceCommand, state: &mut ::Oreos::hal::DeviceState<__DeviceState>, config: &::Oreos::hal::DeviceConfig<__DeviceConfig>) {
                ::Oreos::defmt::debug!("middleware: dispatching command");
                match cmd {
                    #(#on_statements),*
                }
            }
        }
    };

    Ok(TokenStream::from(expanded))
}

