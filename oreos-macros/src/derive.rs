
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, DeriveInput,ItemImpl,Token, Expr::Field, ItemStruct, parse_macro_input, spanned::Spanned, token::Token};


pub fn derive_middleware(input: DeriveInput) -> syn::Result<TokenStream> {

    let name = &input.ident;

    let expanded = quote! {
        impl ::Oreos::hal::Middleware<
            ::Oreos::hal::DeviceState<__DeviceState>,
            ::Oreos::hal::DeviceConfig<__DeviceConfig>,
            __DeviceCommand
        > for #name {
            fn command(&mut self, cmd: __DeviceCommand, state: &mut ::Oreos::hal::DeviceState<__DeviceState>, config: &::Oreos::hal::DeviceConfig<__DeviceConfig>) {
                self.callback_match(cmd, state, config);
            }

            fn process(&mut self, state: &mut ::Oreos::hal::DeviceState<__DeviceState>, config: &::Oreos::hal::DeviceConfig<__DeviceConfig>) {
                let _ = (state, config);
            }
        }
    };

    Ok(TokenStream::from(expanded))
}





pub fn derive_config(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let _data = &input.data;

    let expanded = quote! {
        pub type __DeviceConfig = #name;

        impl ::Oreos::hal::Config for #name {}
    };

    TokenStream::from(expanded)
}


pub fn derive_state(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let _data = &input.data;

    let expanded = quote! {
        pub type __DeviceState = #name;

        impl ::Oreos::hal::State for #name {}
    };

    TokenStream::from(expanded)
}

pub fn impl_kernel(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let data = &input.data;

    let mut state: Option<syn::Ident> = None;
    let mut config: Option<syn::Ident> = None;
    let mut bus: Option<syn::Ident> = None;

    let mut state_type: Option<syn::Type> = None;
    let mut config_type: Option<syn::Type> = None;
    let mut bus_type: Option<syn::Type> = None;


    if let syn::Data::Struct(content) = data {
        for field in &content.fields {
            if field.attrs.iter().any(|attr| attr.path().is_ident("state")) {
                state = Some(field.ident.as_ref().unwrap().clone());
                state_type = Some(field.ty.clone())
            }
            if field.attrs.iter().any(|attr| attr.path().is_ident("config")) {
                config = Some(field.ident.as_ref().unwrap().clone());
                config_type = Some(field.ty.clone())

            }
            if field.attrs.iter().any(|attr| attr.path().is_ident("bus")) {
                bus = Some(field.ident.as_ref().unwrap().clone());
                bus_type = Some(field.ty.clone());
            }
        }
    } else {
        return Err(syn::Error::new_spanned(name, "cannot derive a struct"));
    }

    let state = state.ok_or_else(|| syn::Error::new_spanned(name, "missing #[state] attribute"))?;
    let config = config.ok_or_else(|| syn::Error::new_spanned(name, "missing #[config] attribute"))?;
    let bus = bus.ok_or_else(|| syn::Error::new_spanned(name, "missing #[bus] attribute"))?;
    let state_type = state_type.ok_or_else(|| syn::Error::new_spanned(name, "missing #[state] field"))?;
    let config_type = config_type.ok_or_else(|| syn::Error::new_spanned(name, "missing #[config] field"))?;
    let bus_type = bus_type.ok_or_else(|| syn::Error::new_spanned(name, "missing #[bus] field"))?;


    let expanded = quote! {
        impl #name {
            pub fn new(
                #state: #state_type,
                #config: #config_type,
                #bus: #bus_type,
            ) -> Self {
                Self {
                    #state,
                    #config,
                    #bus,
                }
            }
        }

        impl ::Oreos::hal::Kernel for #name {

            type State = #state_type;
            type Config = #config_type;

            fn init(&mut self, _config: &Self::Config) -> Result<(), ::Oreos::hal::KernelError> {
                Ok(())
            }

            fn feedback(&self) -> Self::State {
                self.#state
            }

            fn tick(&mut self) {
                if self.#bus.estop().is_set() {
                    return;
                }
                self.#bus.update(&mut self.#state);
                self.#bus.write(&self.#state);
            }
        }
    };

    Ok(TokenStream::from(expanded))
}


enum RouteArg {
    In {
        state_field: syn::Path,
        lane_field: syn::Path,
    },
    Out {
        state_field: syn::Path,
        lane_field: syn::Path,
    },
}

impl RouteArg {
    fn state_ident(&self) -> proc_macro2::TokenStream {
        let ident = match self {
            RouteArg::In { state_field, .. } | RouteArg::Out { state_field, .. } => {
                &state_field.segments.last().unwrap().ident
            }
        };
        quote! { #ident }
    }

    fn lane_ident(&self) -> proc_macro2::TokenStream {
        let ident = match self {
            RouteArg::In { lane_field, .. } | RouteArg::Out { lane_field, .. } => {
                &lane_field.segments.last().unwrap().ident
            }
        };
        quote! { #ident }
    }
}

struct RouteList {
    field: syn::punctuated::Punctuated<RouteArg, syn::Token![,]>,
}

impl syn::parse::Parse for RouteList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(RouteList {
            field: syn::punctuated::Punctuated::parse_terminated(input)?,
        })
    }
}

impl syn::parse::Parse for RouteArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let state_field: syn::Path = input.parse()?;

        if input.peek(Token![<]) && input.peek2(Token![=]) {
            input.parse::<Token![<]>()?;
            input.parse::<Token![=]>()?;
            let lane_field: syn::Path = input.parse()?;
            Ok(RouteArg::In {
                state_field,
                lane_field,
            })
        } else if input.peek(Token![=]) && input.peek2(Token![>]) {
            input.parse::<Token![=]>()?;
            input.parse::<Token![>]>()?;
            let lane_field: syn::Path = input.parse()?;
            Ok(RouteArg::Out {
                state_field,
                lane_field,
            })
        } else {
            Err(input.error("expected <= or =>"))
        }
    }
}

fn extract_route_list(meta: &syn::Meta) -> syn::Result<Vec<RouteArg>> {
    let mut items: Vec<RouteArg> = Vec::new();
    if let syn::Meta::List(item_list) = meta {
        let parsed_list: RouteList = item_list.parse_args()?;
        items = parsed_list.field.into_iter().collect();
    }
    Ok(items)
}

pub fn impl_bus(input: DeriveInput) -> syn::Result<TokenStream> {

    let name = &input.ident;
    let data = &input.data;

    let mut route_args_by_field: Vec<(syn::Ident, syn::Type, Vec<RouteArg>)> = Vec::new();

    let mut local_state: Option<syn::Type> = None;
    let mut local_state_ident: Option<syn::Ident> = None;

    if let syn::Data::Struct(content) = data {
        for field in content.fields.iter() {
            let field_name = field.ident.as_ref().unwrap();
            for a in &field.attrs {
                if a.meta.path().is_ident("route") {
                    let routes = extract_route_list(&a.meta)?;
                    route_args_by_field.push((field_name.clone(), field.ty.clone(), routes));
                }

                if a.meta.path().is_ident("state") {
                    local_state_ident = Some(field.ident.as_ref().unwrap().clone());
                    local_state = Some(field.ty.clone());
                }
            }
        }
    } else {
        return Err(syn::Error::new_spanned(name, "impl_bus only supports structs"));
    }

    let static_bus = quote::format_ident!("__{}__", name);

    let local_state = local_state.ok_or_else(|| syn::Error::new_spanned(name, "missing #[state] field"))?;
    let local_state_ident = local_state_ident.ok_or_else(|| syn::Error::new_spanned(name, "missing #[state] field"))?;

    // Separate routes into update (<=) and write (=>)
    let mut update_code_vec = Vec::new();
    let mut write_code_vec = Vec::new();

    for (lane_field, _, routes) in &route_args_by_field {
        let in_routes: Vec<_> = routes.iter().filter(|r| matches!(r, RouteArg::In { .. })).collect();
        let out_routes: Vec<_> = routes.iter().filter(|r| matches!(r, RouteArg::Out { .. })).collect();

        // Generate update code for <= (state <= lane)
        if !in_routes.is_empty() {
            let update_stmts = in_routes.iter().map(|route| {
                let state_ident = route.state_ident();
                let lane_ident = route.lane_ident();
                quote! {
                    state.#state_ident = data.#lane_ident;
                }
            });
            update_code_vec.push(quote! {
                if let Some(data) = self.#lane_field.read() {
                    #(#update_stmts)*
                }
            });
        }

        // Generate write code for => (state => lane)
        if !out_routes.is_empty() {
            let write_stmts = out_routes.iter().map(|route| {
                let state_ident = route.state_ident();
                let lane_ident = route.lane_ident();
                quote! {
                    data.#lane_ident = state.#state_ident;
                }
            });
            write_code_vec.push(quote! {
                if let Some(mut data) = self.#lane_field.read() {
                    #(#write_stmts)*
                    self.#lane_field.write(data);
                }
            });
        }
    }

    let lane_fields: Vec<_> = route_args_by_field.iter().map(|(f, _, _)| f.clone()).collect();
    let lanes_types: Vec<_> = route_args_by_field.iter().map(|(_, t, _)| t.clone()).collect();

    let expanded = quote! {
        impl #name {
            pub fn new(
                #(#lane_fields: #lanes_types,)*
                #local_state_ident: #local_state,
                estop: ::Oreos::hal::EstopFlag,
            ) -> &'static Self {

                static #static_bus: ::static_cell::StaticCell<#name> = ::static_cell::StaticCell::new();
                #static_bus.init(Self {
                    #(#lane_fields,)*
                    #local_state_ident,
                    estop,
                })
            }
        }

        impl ::Oreos::hal::GenericBus<#local_state> for #name {
            fn estop(&self) -> &::Oreos::hal::EstopFlag {
                &self.estop
            }

            fn update(&self, state: &mut #local_state) {
                #(#update_code_vec)*
            }

            fn write(&self, state: &#local_state) {
                #(#write_code_vec)*
            }
        }

        impl #name {
            #(fn #lane_fields(&'static self) -> &'static #lanes_types {
                &self.#lane_fields
            })*
        }
    };

    Ok(TokenStream::from(expanded))
}


// #[create(Device)]
pub fn create_device(mut input: DeriveInput) -> syn::Result<TokenStream> {

    let name = &input.ident;

    let mut backends: Vec<proc_macro2::TokenStream> = Vec::new();

    let mut kernel_type: Option<syn::Type> = None;
    let mut kernel_ident: Option<syn::Ident> = None;

    let mut state_type: Option<syn::Type> = None;
    let mut state_ident: Option<syn::Ident> = None;

    let mut config_type: Option<syn::Type> = None;
    let mut config_ident: Option<syn::Ident> = None;

    let mut middleware_ident: Option<syn::Ident> = None;
    let mut middleware_type: Option<syn::Type> = None;

    let mut is_default_middleware = false;

    // let mut command_ident:Option<syn::Ident> = None;
    // let mut command_type: Option<syn::Type> = None;

    let mut backend_types: Vec<_> = Vec::new();
    let mut backend_tasks: Vec<_> = Vec::new();
    let mut backend_names: Vec<_> = Vec::new();
    let mut field_names: Vec<_> = Vec::new();

    let mut shadowed: proc_macro2::TokenStream = proc_macro2::TokenStream::new();

    // Validate required fields before generating any code


    if let syn::Data::Struct(ref mut data_struct) = input.data {

        for field in data_struct.fields.iter_mut() {

            if field.attrs.iter().any(|attr| attr.path().is_ident("backend")) {
                let raw_ty = field.ty.clone();
                let field_name = field.ident.as_ref().unwrap();

                field.attrs.retain(|a| !a.path().is_ident("backend"));

                let backend_name = quote::format_ident!("__{}_BACKEND__", field_name.to_string().to_uppercase());
                let task_name = quote::format_ident!("__{}_TASK__", field_name.to_string().to_uppercase());

                backend_names.push(backend_name.clone());
                backend_tasks.push(task_name.clone());
                backend_types.push(raw_ty.clone());
                field_names.push(field_name.clone());

                field.vis = syn::parse_quote!(pub);

                field.ty = syn::parse_quote!(
                    ::core::cell::UnsafeCell<Option<#raw_ty>>
                );

                backends.push(quote! {
                    static #backend_name: ::static_cell::StaticCell<#raw_ty> = ::static_cell::StaticCell::new();

                    #[embassy_executor::task]
                    async fn #task_name(backend: &'static mut #raw_ty) {
                        loop {
                            backend.tick().await;
                            //temporary 
                            ::Oreos::embassy_time::Timer::after_millis(10).await;
                        }
                    }
                });
            }

            if field.attrs.iter().any(|attr| attr.path().is_ident("middleware")) {
                field.attrs.retain(|a| !a.path().is_ident("middleware"));

                middleware_ident = Some(field.ident.as_ref().unwrap().clone());
                middleware_type = Some(field.ty.clone())
            }

            if field.attrs.iter().any(|attr| attr.path().is_ident("kernel")) {
                field.attrs.retain(|a| !a.path().is_ident("kernel"));

                kernel_ident = Some(field.ident.as_ref().unwrap().clone());
                kernel_type = Some(field.ty.clone());
            }

            if field.attrs.iter().any(|attr| attr.path().is_ident("state")) {
                field.attrs.retain(|a| !a.path().is_ident("state"));

                state_ident = Some(field.ident.as_ref().unwrap().clone());
                state_type = Some(field.ty.clone());
            }

            if field.attrs.iter().any(|attr| attr.path().is_ident("config")) {
                field.attrs.retain(|a| !a.path().is_ident("config"));

                config_ident = Some(field.ident.as_ref().unwrap().clone());
                config_type = Some(field.ty.clone());
            }


            // if field.attrs.iter().any(|attr| attr.path().is_ident("command")) {
            //     field.attrs.retain(|a| !a.path().is_ident("command"));

            //     command_ident = Some(field.ident.as_ref().unwrap().clone());
            //     command_type = Some(field.ty.clone());
            // }

    
        }


        if middleware_ident.is_none() {
            is_default_middleware = true;
            let middleware_field = syn::Field {
                attrs: Vec::new(),
                vis: syn::Visibility::Inherited,
                ident: Some(syn::Ident::new("__NO_MIDDLEWARE__", proc_macro2::Span::call_site())),
                mutability: syn::FieldMutability::None,
                colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
                ty: syn::parse_quote!(::Oreos::hal::NoMiddleware)
            };

            if let syn::Fields::Named(ref mut fields) = data_struct.fields {
                fields.named.push(middleware_field.clone());
            }
            middleware_type = Some(middleware_field.ty.clone());
            middleware_ident = Some(middleware_field.ident.as_ref().unwrap().clone());

        }
    } else {
        proc_macro_error::abort!(
            name, "#[create(Device)] only works on structs"
        );
    } 

    let kernel_type = kernel_type.ok_or_else(|| syn::Error::new_spanned(name, "missing #[kernel] field"))?;
    let kernel_ident = kernel_ident.ok_or_else(|| syn::Error::new_spanned(name, "missing #[kernel] field"))?;
    let state_type = state_type.ok_or_else(|| syn::Error::new_spanned(name, "missing #[state] field"))?;
    let state_ident = state_ident.ok_or_else(|| syn::Error::new_spanned(name, "missing #[state] field"))?;
    let config_type = config_type.ok_or_else(|| syn::Error::new_spanned(name, "missing #[config] field"))?;
    let config_ident = config_ident.ok_or_else(|| syn::Error::new_spanned(name, "missing #[config] field"))?;
    
    let middleware_init = if is_default_middleware {
        quote! {
            #middleware_ident: ::Oreos::hal::NoMiddleware::default()
        }
    } else {
        quote! {
            #middleware_ident
        }
    };

    let middleware_param = if is_default_middleware {
        quote! {}
    } else {
        let middleware_type = middleware_type.as_ref().unwrap();
        quote! {
            #middleware_ident: #middleware_type,
        }
    };

    shadowed = quote! {
        impl #name {
            pub fn new(
                #(#field_names: #backend_types,)*
                #kernel_ident: #kernel_type,
                #state_ident: #state_type,
                #config_ident: #config_type,
                #middleware_param
            ) -> Self {
                ::Oreos::defmt::info!("creating device: {}", stringify!(#name));
                Self {
                    #(#field_names: ::core::cell::UnsafeCell::new(Some(#field_names)),)*
                    #kernel_ident,
                    #state_ident,
                    #config_ident,
                    #middleware_init
                }
            }
        }
    };

    let expanded = quote! {
        #input

        #shadowed



        impl ::Oreos::hal::Device for #name {
            type Kernel = #kernel_type;
            type Command = __DeviceCommand;

            fn tick(&mut self, _dt: ::Oreos::fugit::Duration<u32, 1, 1000>) {
                ::Oreos::defmt::trace!("device tick: {}", stringify!(#name));

                self.#kernel_ident.state = self.#state_ident.custom;
                self.#kernel_ident.tick();
                self.#state_ident.custom = self.#kernel_ident.feedback();
                self.#middleware_ident.process(&mut self.#state_ident, &self.#config_ident);
            }

            fn kernel(&mut self) -> &mut Self::Kernel {
                &mut self.#kernel_ident
            }

            fn execute(&mut self, cmd: Self::Command) {
                ::Oreos::defmt::debug!("device execute command: {}", stringify!(#name));
                self.#middleware_ident.command(cmd, &mut self.#state_ident, &self.#config_ident);
            }
        }


        #(#backends)*


        impl ::Oreos::hal::MaybeDevcie for #name {
            fn start(&'static self, spawner: ::embassy_executor::Spawner) {


                #(let #field_names = unsafe { &mut *self.#field_names.get() }.take().unwrap();)*

                #(let #field_names = #backend_names.init(#field_names);)*

                #(spawner.spawn(#backend_tasks(#field_names).unwrap());)*

                



                // #(#spawn_calls)*
                // Objective
                // impl MaybeDevice for ArmJoint {
                //     fn start(&'static self, spawner: Spawner) {
                //         // take out of UnsafeCell<Option<T>>
                //         let motor = unsafe { &mut *self.motor.get() }.take().unwrap();
                //         let driver = unsafe { &mut *self.driver.get() }.take().unwrap();

                //         // move into statics
                //         let motor = __MOTOR__.init(motor);
                //         let driver = __DRIVER__.init(driver);

                //         spawner.spawn(motor_task(motor)).unwrap();
                //         spawner.spawn(driver_task(driver)).unwrap();
                //     }
                // }
            }
        }

        



    };

    Ok(TokenStream::from(expanded))

}

