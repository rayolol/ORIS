use std::fmt::Result;

use proc_macro2::{self, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, Ident, Token, Type};

struct IoAccessArgs {
    ident: Ident,
    token: Token![=],
    kind: String,
}

enum IoKind {
    Pwm,
    Analog,
    Digital,
    Transport,
}

struct IoField {
    ident: Ident,
    ty: Type,
    kind: IoKind,
}

impl syn::parse::Parse for IoAccessArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let token: Token![=] = input.parse()?;
        let kind: String = input.parse::<syn::LitStr>()?.value();

        Ok(IoAccessArgs { ident, token, kind })
    }
}

pub fn create_access_io(mut input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let access_name = &input.ident;

    let mut io_fields: Vec<IoField> = Vec::new();

    let content = match &input.data {
        syn::Data::Struct(content) => content,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "IoAccess can only be derived for structs",
            ));
        }
    };

    for field in &content.fields {
        let Some(ident) = field.ident.clone() else {
            continue;
        };
        for attr in &field.attrs {
            if attr.path().is_ident("io") {
                let args: IoAccessArgs = attr.parse_args()?;

                match args.kind.as_str() {
                    "pwm" => io_fields.push(IoField {
                        ident: ident.clone(),
                        ty: field.ty.clone(),
                        kind: IoKind::Pwm,
                    }),

                    "analog" => io_fields.push(IoField {
                        ident: ident.clone(),
                        ty: field.ty.clone(),
                        kind: IoKind::Analog,
                    }),

                    "digital" => io_fields.push(IoField {
                        ident: ident.clone(),
                        ty: field.ty.clone(),
                        kind: IoKind::Digital,
                    }),

                    "transport" => io_fields.push(IoField {
                        ident: ident.clone(),
                        ty: field.ty.clone(),
                        kind: IoKind::Transport,
                    }),
                    _ => {}
                }
            }
        }
    }

    let structured_fields: Vec<TokenStream> = io_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;

            quote! {
                #ident: #ty
            }
        })
        .collect();

    let getters: Vec<TokenStream> = io_fields
        .iter()
        .map(|f| {
            let field_ident = &f.ident;
            let ty = &f.ty;
            let getter = format_ident!("{}_mut", field_ident);

            quote! {
                pub fn #getter(&mut self) -> &mut #ty {
                    &mut self.#field_ident
                }
            }
        })
        .collect();

    let mut generics = input.generics.clone();

    for field in &io_fields {
        match &field.kind {
            IoKind::Pwm => {
                let ty = &field.ty;

                generics
                    .make_where_clause()
                    .predicates
                    .push(syn::parse_quote!(
                        #ty: ::embedded_hal::pwm::SetDutyCycle,
                    ));
            }
            IoKind::Analog => {}
            IoKind::Digital => {
                let ty = &field.ty;

                generics
                    .make_where_clause()
                    .predicates
                    .push(syn::parse_quote!(
                        #ty: ::embedded_hal::digital::InputPin + ::embedded_hal::digital::OutputPin,
                    ));
            }
            IoKind::Transport => {}
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let returns: Vec<&Ident> = io_fields.iter().map(|f| &f.ident).collect();

    let out = quote! {

        impl #impl_generics IoAccess for #access_name #type_generics  {}

        impl #impl_generics #access_name #type_generics #where_clause {
            fn new(#(#structured_fields),*) -> Self {
                Self {
                    #(#returns),*
                }
            }

            #(#getters)*
        }
    };

    Ok(out.to_token_stream())
}
