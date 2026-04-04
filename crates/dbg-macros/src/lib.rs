//! Procedural macros for the `dbgflow` graph debugger.
//!
//! This crate is usually consumed indirectly through the top-level `dbgflow`
//! crate, which re-exports all macros.
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{ToTokens, quote};
use syn::{Attribute, Item, ItemEnum, ItemFn, ItemStruct, parse_macro_input};

/// Marks a function as a traced execution node.
///
/// The generated code records function entry, argument previews, and the final
/// return event into the active session.
#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[trace] does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut function = parse_macro_input!(item as ItemFn);
    let ident = function.sig.ident.clone();
    let dbgflow = dbgflow_crate_path();

    let argument_values = function.sig.inputs.iter().map(|arg| match arg {
        syn::FnArg::Receiver(_) => {
            quote! { #dbgflow::runtime::preview_argument("self", &self) }
        }
        syn::FnArg::Typed(pat_type) => {
            let pat = &pat_type.pat;
            let name = pat.to_token_stream().to_string();
            match pat.as_ref() {
                syn::Pat::Ident(pat_ident) => {
                    let binding = &pat_ident.ident;
                    quote! { #dbgflow::runtime::preview_argument(#name, &#binding) }
                }
                _ => quote! {
                    #dbgflow::ValueSlot {
                        name: #name.to_owned(),
                        preview: "<non-ident pattern>".to_owned(),
                    }
                },
            }
        }
    });

    let block = &function.block;
    function.block = Box::new(syn::parse_quote!({
        let mut __dbg_frame = #dbgflow::runtime::TraceFrame::enter(
            #dbgflow::FunctionMeta {
                id: concat!(module_path!(), "::", stringify!(#ident)),
                label: stringify!(#ident),
                module_path: module_path!(),
                file: file!(),
                line: line!(),
            },
            vec![#(#argument_values),*],
        );
        let __dbg_result = { #block };
        __dbg_frame.finish_return(&__dbg_result);
        __dbg_result
    }));

    quote!(#function).into()
}

/// Marks a struct or enum as a UI-visible data node.
///
/// Types annotated with `#[ui_debug]` implement `dbgflow::UiDebugValue` and can
/// emit snapshots with `value.emit_snapshot("label")`.
#[proc_macro_attribute]
pub fn ui_debug(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[ui_debug] does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    let item = parse_macro_input!(item as Item);
    match item {
        Item::Struct(item_struct) => expand_struct(item_struct).into(),
        Item::Enum(item_enum) => expand_enum(item_enum).into(),
        _ => syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[ui_debug] supports structs and enums only",
        )
        .to_compile_error()
        .into(),
    }
}

/// Wraps a test so it becomes a persisted debugger session.
///
/// The macro initializes a fresh session, records test start and finish events,
/// persists the session if `DBG_SESSION_DIR` is set, and rethrows panics so the
/// underlying test outcome remains unchanged.
#[proc_macro_attribute]
pub fn dbg_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[dbg_test] does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut function = parse_macro_input!(item as ItemFn);
    let ident = function.sig.ident.clone();
    let dbgflow = dbgflow_crate_path();

    if function.sig.asyncness.is_some() {
        return syn::Error::new_spanned(
            &function.sig.ident,
            "#[dbg_test] does not support async tests yet",
        )
        .to_compile_error()
        .into();
    }

    if !function
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("test"))
    {
        function.attrs.push(syn::parse_quote!(#[test]));
    }

    let test_name = format!("{}", ident);
    let block = &function.block;
    function.block = Box::new(syn::parse_quote!({
        let __dbg_test_name = concat!(module_path!(), "::", #test_name);
        #dbgflow::init_session(format!("dbgflow test: {}", __dbg_test_name));
        #dbgflow::runtime::record_test_started_latest(__dbg_test_name);

        let __dbg_result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #block));
        match __dbg_result {
            Ok(__dbg_value) => {
                #dbgflow::runtime::record_test_passed_latest(__dbg_test_name);
                let _ = #dbgflow::persist_session_from_env(__dbg_test_name);
                __dbg_value
            }
            Err(__dbg_panic) => {
                #dbgflow::runtime::record_test_failed_latest(
                    __dbg_test_name,
                    #dbgflow::panic_message(&*__dbg_panic),
                );
                let _ = #dbgflow::persist_session_from_env(__dbg_test_name);
                ::std::panic::resume_unwind(__dbg_panic)
            }
        }
    }));

    quote!(#function).into()
}

fn expand_struct(mut item: ItemStruct) -> proc_macro2::TokenStream {
    maybe_add_debug_derive(&mut item.attrs);
    let ident = &item.ident;
    let dbgflow = dbgflow_crate_path();

    quote! {
        #item

        impl #dbgflow::UiDebugValue for #ident {
            fn ui_debug_type_meta() -> #dbgflow::TypeMeta {
                #dbgflow::TypeMeta {
                    id: concat!(module_path!(), "::", stringify!(#ident)),
                    label: stringify!(#ident),
                    module_path: module_path!(),
                    file: file!(),
                    line: line!(),
                }
            }
        }
    }
}

fn expand_enum(mut item: ItemEnum) -> proc_macro2::TokenStream {
    maybe_add_debug_derive(&mut item.attrs);
    let ident = &item.ident;
    let dbgflow = dbgflow_crate_path();

    quote! {
        #item

        impl #dbgflow::UiDebugValue for #ident {
            fn ui_debug_type_meta() -> #dbgflow::TypeMeta {
                #dbgflow::TypeMeta {
                    id: concat!(module_path!(), "::", stringify!(#ident)),
                    label: stringify!(#ident),
                    module_path: module_path!(),
                    file: file!(),
                    line: line!(),
                }
            }
        }
    }
}

fn maybe_add_debug_derive(attrs: &mut Vec<Attribute>) {
    let has_debug = attrs.iter().any(|attr| {
        attr.path().is_ident("derive") && attr.meta.to_token_stream().to_string().contains("Debug")
    });

    if !has_debug {
        attrs.push(syn::parse_quote!(#[derive(Debug)]));
    }
}

fn dbgflow_crate_path() -> proc_macro2::TokenStream {
    match crate_name("dbgflow") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::dbgflow),
    }
}
