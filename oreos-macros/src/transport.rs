use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

// TODO change macro method to generate transports
struct HelperArgs {
    expr: syn::Expr,
}

impl syn::parse::Parse for HelperArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let expr: syn::Expr = input.parse()?;
        return Ok(HelperArgs { expr });
    }
}

pub fn make_transport(input: TokenStream) -> TokenStream {
    let items = parse_macro_input!(input as HelperArgs);

    let periph = items.expr;

    let output = quote! {
        {

            static __TRANSPORT_BUFFER__: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, PacketRequest, 10> = embassy_sync::channel::Channel::new();

            let server = TransportServer::new(
                #periph,
                &__TRANSPORT_BUFFER__
            );

            let client = TransportClient::new(
                &__TRANSPORT_BUFFER__
            );

            (client, server)
        }
    };
    output.into()
}
