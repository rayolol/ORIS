use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use crate::DeviceItem;

use super::DeviceList;


pub fn devices(_app_attr: TokenStream, input: TokenStream) -> TokenStream {
    let list = parse_macro_input!(input as DeviceList);

    // #[device] items implement MaybeDevice and need .start(spawner) called on them.
    // They are stored exclusively (like nonshared) but are not user-accessible via ContextView.
    let is_device    = |i: &&DeviceItem| i.attrs.iter().any(|a| a.path().is_ident("device"));
    let is_shared    = |i: &&DeviceItem| i.shared && !is_device(i);
    let is_nonshared = |i: &&DeviceItem| !i.shared && !is_device(i);

    let shared_items:    Vec<&DeviceItem> = list.items.iter().filter(is_shared).collect();
    let nonshared_items: Vec<&DeviceItem> = list.items.iter().filter(is_nonshared).collect();
    let device_items:    Vec<&DeviceItem> = list.items.iter().filter(is_device).collect();

    let shared_names:    Vec<_> = shared_items.iter().map(|i| &i.name).collect();
    let shared_types:    Vec<_> = shared_items.iter().map(|i| &i.ty).collect();
    let shared_statics:  Vec<_> = shared_names.iter().map(|n| {
        quote::format_ident!("__{}__", n.to_string().to_uppercase())
    }).collect();

    let nonshared_names:   Vec<_> = nonshared_items.iter().map(|i| &i.name).collect();
    let nonshared_types:   Vec<_> = nonshared_items.iter().map(|i| &i.ty).collect();
    let nonshared_statics: Vec<_> = nonshared_names.iter().map(|n| {
        quote::format_ident!("__{}__", n.to_string().to_uppercase())
    }).collect();

    let device_names:   Vec<_> = device_items.iter().map(|i| &i.name).collect();
    let device_types:   Vec<_> = device_items.iter().map(|i| &i.ty).collect();
    let device_statics: Vec<_> = device_names.iter().map(|n| {
        quote::format_ident!("__{}__", n.to_string().to_uppercase())
    }).collect();
    // Temporary idents used inside __init_devices__ to hold the static shared ref
    let device_static_refs: Vec<_> = device_names.iter().map(|n| {
        quote::format_ident!("__{}_static_ref__", n.to_string())
    }).collect();

    let out = quote! {
        pub mod devices {
            use super::*;

            // Shared devices — wrapped in SharedCell for multi-task access
            #(
                pub static #shared_statics: ::static_cell::StaticCell<
                    ::shared_cell::SharedCell<#shared_types>
                > = ::static_cell::StaticCell::new();
            )*

            // Non-shared devices — exclusively owned, raw pointer access
            #(
                pub static #nonshared_statics: ::static_cell::StaticCell<#nonshared_types>
                    = ::static_cell::StaticCell::new();
            )*

            // MaybeDevice items — stored in statics so start() gets a &'static self
            #(
                pub static #device_statics: ::static_cell::StaticCell<#device_types>
                    = ::static_cell::StaticCell::new();
            )*

            // StoredContext — Copy, lives in each task's static storage.
            // Non-shared and device fields are raw pointers (exclusive, no lock needed).
            // Shared devices are &'static SharedCell<T> (multiple tasks, critical section).
            #[derive(Copy, Clone)]
            pub struct StoredContext {
                #( pub #nonshared_names: *mut #nonshared_types, )*
                #( pub #device_names: *mut #device_types, )*
                #( pub #shared_names: &'static ::shared_cell::SharedCell<#shared_types>, )*
            }
            unsafe impl Send for StoredContext {}
            unsafe impl Sync for StoredContext {}

            // ContextView — user-facing. Non-shared and device fields are &mut T (direct access).
            // Shared fields are Shared<T> which implements Deref/DerefMut, so
            // ctx.field.method() works transparently and the IDE shows the real type.
            pub struct ContextView<'__ctx> {
                #( pub #nonshared_names: &'__ctx mut #nonshared_types, )*
                #( pub #device_names: &'__ctx mut #device_types, )*
                #( pub #shared_names: ::Shared<#shared_types>, )*
                pub(crate) _phantom: ::core::marker::PhantomData<&'__ctx ()>,
            }

            impl StoredContext {
                pub fn view(&mut self) -> ContextView<'_> {
                    ContextView {
                        #( #nonshared_names: unsafe { &mut *self.#nonshared_names }, )*
                        #( #device_names: unsafe { &mut *self.#device_names }, )*
                        #( #shared_names: crate::Shared(self.#shared_names), )*
                        _phantom: ::core::marker::PhantomData,
                    }
                }
            }

            pub struct Devices {
                #( pub #shared_names: #shared_types, )*
                #( pub #nonshared_names: #nonshared_types, )*
                #( pub #device_names: #device_types, )*
            }

            pub fn __init_devices__(
                spawner: ::embassy_executor::Spawner,
                device: Devices,
            ) -> StoredContext {
                // Move each MaybeDevice into its static.
                // Get the raw pointer first, then reconstruct a &'static ref for start() —
                // avoids a borrow conflict between the 'static shared borrow and *mut.
                #(
                    let #device_names: *mut #device_types =
                        #device_statics.init(device.#device_names) as *mut _;
                    let #device_static_refs: &'static #device_types =
                        unsafe { &*(#device_names as *const #device_types) };
                    #device_static_refs.start(spawner);
                )*

                StoredContext {
                    #( #nonshared_names: #nonshared_statics.init(device.#nonshared_names) as *mut _, )*
                    #( #device_names, )*
                    #( #shared_names: #shared_statics.init(
                        ::shared_cell::SharedCell::new(device.#shared_names)
                    ), )*
                }
            }
        }

        pub use devices::ContextView as Context;
        pub use devices::StoredContext;
        pub use devices::Devices;
    };

    proc_macro::TokenStream::from(out)
}
