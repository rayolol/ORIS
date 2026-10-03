use proc_macro2::{self, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, Ident, Token, Type};

struct IoAccessArgs {
    kind: IoKind,
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
        if ident != "kind" {
            return Err(syn::Error::new(ident.span(), "expected `kind`"));
        }

        let _: Token![=] = input.parse()?;
        let kind = input.parse::<syn::LitStr>()?;

        let kind = match kind.value().as_str() {
            "pwm" => IoKind::Pwm,
            "analog" => IoKind::Analog,
            "digital" => IoKind::Digital,
            "transport" => IoKind::Transport,
            unknown => Err(syn::Error::new(
                kind.span(),
                format!(
                    "unknown I/O kind `{unknown}`; expected `pwm`, `analog`, `digital`, or `transport`"
                ),
            ))?,
        };

        Ok(Self { kind })
    }
}

pub fn create_access_io(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
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
                let args = attr.parse_args::<IoAccessArgs>()?;
                io_fields.push(IoField {
                    ident: ident.clone(),
                    ty: field.ty.clone(),
                    kind: args.kind,
                });
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
        if let IoKind::Pwm = field.kind {
            let ty = &field.ty;
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(#ty: ::embedded_hal::pwm::SetDutyCycle));
        }
    }

    // Digital direction belongs to the backend that consumes the resource: an
    // ordinary pin may be input-only or output-only.
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let returns: Vec<&Ident> = io_fields.iter().map(|f| &f.ident).collect();

    let out = quote! {

        impl #impl_generics IoAccess for #access_name #type_generics #where_clause {}

        impl #impl_generics #access_name #type_generics #where_clause {
            pub fn new(#(#structured_fields),*) -> Self {
                Self {
                    #(#returns),*
                }
            }

            #(#getters)*
        }
    };

    Ok(out.to_token_stream())
}

#[cfg(test)]
mod tests {
    use super::IoAccessArgs;

    #[test]
    fn rejects_unknown_io_kind() {
        let error = match syn::parse_str::<IoAccessArgs>(r#"kind = "serial""#) {
            Ok(_) => panic!("unknown I/O kind was accepted"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "unknown I/O kind `serial`; expected `pwm`, `analog`, `digital`, or `transport`"
        );
    }
}
