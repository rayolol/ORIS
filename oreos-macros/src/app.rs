use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemMod, parse_macro_input};

use super::AppArgs;

const DEFAULT_LOOP_RATE: u64 = 1000;
const DEFAULT_PRIORITY: u8 = 1;

struct LoopArgs {
    rate: u64,
    priority: u8,
}

impl Default for LoopArgs {
    fn default() -> Self {
        Self {
            rate: DEFAULT_LOOP_RATE,
            priority: DEFAULT_PRIORITY,
        }
    }
}

fn parse_int<T: std::str::FromStr>(expr: &syn::Expr) -> syn::Result<T>
where
    T::Err: std::fmt::Display,
{
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(lit),
        ..
    }) = expr
    {
        lit.base10_parse::<T>()
    } else {
        Err(syn::Error::new_spanned(expr, "Expected an integer"))
    }
}

/// Rewrites a `ctx: Context` (or `&Context` / `&mut Context`) parameter type
/// to `crate::devices::ContextView<'_>` so the IDE sees the actual device types.
fn rewrite_ctx_param_type(function: &mut syn::ItemFn) {
    for input in function.sig.inputs.iter_mut() {
        if let syn::FnArg::Typed(pat_type) = input {
            let is_context = match &*pat_type.ty {
                syn::Type::Path(tp) => tp.path.is_ident("Context"),
                syn::Type::Reference(r) => {
                    matches!(&*r.elem, syn::Type::Path(tp) if tp.path.is_ident("Context"))
                }
                _ => false,
            };
            if is_context {
                *pat_type.ty = syn::parse_quote!(crate::devices::ContextView<'_>);
                // Must be mut so DerefMut can auto-ref the Shared<T> fields
                if let syn::Pat::Ident(pat_ident) = &mut *pat_type.pat {
                    pat_ident.mutability = Some(syn::token::Mut::default());
                }
            }
        }
    }
}

pub fn app(app_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut init_fn: Option<syn::Ident> = None;

    let mut input = parse_macro_input!(item as ItemMod);
    let args = parse_macro_input!(app_attr as AppArgs);
    let hal_crate = &args.hal_crate;

    let mut generated_tasks = Vec::new();
    let mut spawn_calls = Vec::new();
    let mut config: Option<syn::Expr> = None;

    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;

    if let Some((_, items)) = &mut input.content {
        items.insert(
            0,
            syn::parse_quote!(
                use super::*;
            ),
        );
        items.insert(
            1,
            syn::parse_quote!(
                use crate::devices::*;
            ),
        );

        for item in items.iter_mut() {
            if let syn::Item::Fn(function) = item {
                let is_loop = function.attrs.iter().any(|a| a.path().is_ident("loop_"));
                let is_init = function.attrs.iter().any(|a| a.path().is_ident("init"));
                function.vis = syn::Visibility::Public(syn::token::Pub::default());

                if is_init {
                    let attr = function
                        .attrs
                        .iter()
                        .find(|a| a.path().is_ident("init"))
                        .unwrap();

                    let args = match attr.parse_args_with(parser) {
                        Ok(a) => a,
                        Err(e) => return e.to_compile_error().into(),
                    };

                    if init_fn.is_some() {
                        return syn::Error::new_spanned(function, "only one #[init] is allowed")
                            .to_compile_error()
                            .into();
                    }

                    for meta in args {
                        match meta {
                            syn::Meta::NameValue(nv) if nv.path.is_ident("config") => {
                                config = Some(nv.value.clone());
                            }
                            _ => {
                                return syn::Error::new_spanned(meta, "Expected key = value")
                                    .to_compile_error()
                                    .into();
                            }
                        }
                    }

                    init_fn = Some(function.sig.ident.clone());
                    function.attrs.retain(|a| !a.path().is_ident("init"));
                }

                if is_loop {
                    let attr = function
                        .attrs
                        .iter()
                        .find(|a| a.path().is_ident("loop_"))
                        .unwrap();

                    let content: Vec<_> = function.sig.inputs.iter().collect();

                    let mut settings = LoopArgs::default();
                    let args = match attr.parse_args_with(parser) {
                        Ok(a) => a,
                        Err(e) => return e.to_compile_error().into(),
                    };

                    let func_args_type: Vec<_> = function
                        .sig
                        .inputs
                        .iter()
                        .filter_map(|a| {
                            if let syn::FnArg::Typed(pt) = a {
                                if let syn::Type::Path(tp) = &*pt.ty {
                                    if tp.path.is_ident("Context") {
                                        return None;
                                    } else {
                                        return Some(pt.clone());
                                    }
                                }
                            }
                            None
                        })
                        .collect();

                    // just the names, for forwarding in the call
                    let func_arg_names: Vec<syn::Ident> = func_args_type
                        .iter()
                        .filter_map(|pt| {
                            if let syn::Pat::Ident(pi) = &*pt.pat {
                                Some(pi.ident.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    for meta in args {
                        match meta {
                            syn::Meta::NameValue(nv) if nv.path.is_ident("rate") => {
                                match parse_int::<u64>(&nv.value) {
                                    Ok(v) => settings.rate = v,
                                    Err(e) => return e.to_compile_error().into(),
                                }
                            }
                            syn::Meta::NameValue(nv) if nv.path.is_ident("priority") => {
                                match parse_int::<u8>(&nv.value) {
                                    Ok(v) => settings.priority = v,
                                    Err(e) => return e.to_compile_error().into(),
                                }
                            }
                            _ => {}
                        }
                    }

                    // Rewrite ctx: Context → ctx: ContextView<'_> in the user function.
                    // ContextView has the actual device types, so the IDE shows correct hints
                    // and method calls work without any body rewriting.
                    rewrite_ctx_param_type(function);

                    let fn_name = function.sig.ident.clone();
                    let task_name = quote::format_ident!("{}_task", fn_name);
                    let rate = settings.rate;
                    let mut once_calls: Vec<syn::Local> = Vec::new();
                    let mut once_indices: Vec<usize> = Vec::new();

                    for (i, stmt) in function.block.stmts.iter_mut().enumerate() {
                        if let syn::Stmt::Local(local) = stmt {
                            let is_once = local.attrs.iter().any(|a| a.path().is_ident("once"));
                            if is_once {
                                local.attrs.retain(|a| !a.path().is_ident("once"));
                                once_calls.push(local.clone());
                                once_indices.push(i);
                            }
                        }
                    }
                    for i in once_indices.into_iter().rev() {
                        function.block.stmts.remove(i);
                    }

                    // Generated task: owns a StoredContext (Copy), creates a ContextView
                    // on each tick and passes it to the user function.
                    let new_task = quote! {
                        #[embassy_executor::task]
                        async fn #task_name(mut ctx: crate::devices::StoredContext) {
                            #(#once_calls);*
                            loop {
                                crate::app::#fn_name(ctx.view()).await;
                                ::Oreos::embassy_time::Timer::after_millis(#rate).await;
                            }
                        }
                    };

                    let spawn_call = quote! {
                        spawner.spawn(#task_name(ctx).unwrap());
                    };

                    function.attrs.retain(|a| !a.path().is_ident("loop_"));
                    generated_tasks.push(new_task);
                    spawn_calls.push(spawn_call);
                }
            }
        }
    }

    let init_call = match init_fn {
        Some(name) => quote! { #name },
        None => syn::Error::new_spanned(&input, "Missing #[init] function").into_compile_error(),
    };

    let init_config = config.unwrap_or_else(|| syn::parse_quote! { Default::default() });

    let expanded = quote! {
        #input

        #[embassy_executor::main]
        async fn main(spawner: embassy_executor::Spawner) {
            use crate::app::*;
            let p = #hal_crate::init(#init_config);
            let dev: devices::Devices = #init_call(p, spawner).await;
            let ctx = devices::__init_devices__(spawner, dev);

            #(#spawn_calls)*
        }

        #(#generated_tasks)*
    };

    TokenStream::from(expanded)
}
