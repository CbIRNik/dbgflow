//! Procedural macros for the `dbgflow` graph debugger.
//!
//! This crate is usually consumed indirectly through the top-level `dbgflow`
//! crate, which re-exports all macros.
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Ident, Item, ItemEnum, ItemFn, ItemStruct, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Marks a function as a traced execution node.
///
/// The generated code records function entry, argument previews, and the final
/// return event into the active session.
///
/// Optional arguments:
/// - `name = "..."` overrides the label shown in the UI.
#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as MacroOptions);

    let mut function = parse_macro_input!(item as ItemFn);
    let original_function = function.clone();
    let ident = function.sig.ident.clone();
    let dbgflow = dbgflow_crate_path();
    let label = options.label_or(&ident);
    let source = formatted_function_source(&original_function);

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

    if function.sig.asyncness.is_some() {
        function.block = Box::new(syn::parse_quote!({
            #dbgflow::runtime::trace_future(
                #dbgflow::FunctionMeta {
                    id: concat!(module_path!(), "::", stringify!(#ident)),
                    label: #label,
                    module_path: module_path!(),
                    file: file!(),
                    line: line!(),
                    source: #source,
                },
                vec![#(#argument_values),*],
                async move { #block }
            ).await
        }));
    } else {
        function.block = Box::new(syn::parse_quote!({
            let mut __dbg_frame = #dbgflow::runtime::TraceFrame::enter(
                #dbgflow::FunctionMeta {
                    id: concat!(module_path!(), "::", stringify!(#ident)),
                    label: #label,
                    module_path: module_path!(),
                    file: file!(),
                    line: line!(),
                    source: #source,
                },
                vec![#(#argument_values),*],
            );
            let __dbg_result = { #block };
            __dbg_frame.finish_return(&__dbg_result);
            __dbg_result
        }));
    }

    quote!(#function).into()
}

/// Marks a struct or enum as a UI-visible data node.
///
/// Types annotated with `#[ui_debug]` implement `dbgflow::UiDebugValue` and can
/// emit snapshots with `value.emit_snapshot("label")`.
///
/// Optional arguments:
/// - `name = "..."` overrides the label shown in the UI.
#[proc_macro_attribute]
pub fn ui_debug(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as MacroOptions);

    let item = parse_macro_input!(item as Item);
    match item {
        Item::Struct(item_struct) => expand_struct(item_struct, options).into(),
        Item::Enum(item_enum) => expand_enum(item_enum, options).into(),
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
///
/// Optional arguments:
/// - `name = "..."` overrides the test node label shown in the UI.
#[proc_macro_attribute]
pub fn dbg_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as MacroOptions);

    let mut function = parse_macro_input!(item as ItemFn);
    let ident = function.sig.ident.clone();
    let dbgflow = dbgflow_crate_path();
    let label = options.label_or(&ident);

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
        #dbgflow::runtime::record_test_started_latest_with_label(__dbg_test_name, #label);

        let __dbg_result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #block));
        match __dbg_result {
            Ok(__dbg_value) => {
                #dbgflow::runtime::record_test_passed_latest_with_label(__dbg_test_name, #label);
                let _ = #dbgflow::persist_session_from_env(__dbg_test_name);
                __dbg_value
            }
            Err(__dbg_panic) => {
                #dbgflow::runtime::record_test_failed_latest_with_label(
                    __dbg_test_name,
                    #label,
                    #dbgflow::panic_message(&*__dbg_panic),
                );
                let _ = #dbgflow::persist_session_from_env(__dbg_test_name);
                ::std::panic::resume_unwind(__dbg_panic)
            }
        }
    }));

    quote!(#function).into()
}

fn expand_struct(mut item: ItemStruct, options: MacroOptions) -> proc_macro2::TokenStream {
    let source = formatted_struct_source(&item);
    maybe_add_debug_derive(&mut item.attrs);
    let ident = &item.ident;
    let dbgflow = dbgflow_crate_path();
    let label = options.label_or(ident);

    quote! {
        #item

        impl #dbgflow::UiDebugValue for #ident {
            fn ui_debug_type_meta() -> #dbgflow::TypeMeta {
                #dbgflow::TypeMeta {
                    id: concat!(module_path!(), "::", stringify!(#ident)),
                    label: #label,
                    module_path: module_path!(),
                    file: file!(),
                    line: line!(),
                    source: #source,
                }
            }
        }
    }
}

fn expand_enum(mut item: ItemEnum, options: MacroOptions) -> proc_macro2::TokenStream {
    let source = formatted_enum_source(&item);
    maybe_add_debug_derive(&mut item.attrs);
    let ident = &item.ident;
    let dbgflow = dbgflow_crate_path();
    let label = options.label_or(ident);

    quote! {
        #item

        impl #dbgflow::UiDebugValue for #ident {
            fn ui_debug_type_meta() -> #dbgflow::TypeMeta {
                #dbgflow::TypeMeta {
                    id: concat!(module_path!(), "::", stringify!(#ident)),
                    label: #label,
                    module_path: module_path!(),
                    file: file!(),
                    line: line!(),
                    source: #source,
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

#[derive(Default)]
struct MacroOptions {
    name: Option<LitStr>,
}

impl MacroOptions {
    fn label_or(&self, fallback: &Ident) -> LitStr {
        self.name
            .clone()
            .unwrap_or_else(|| LitStr::new(&fallback.to_string(), fallback.span()))
    }
}

impl Parse for MacroOptions {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::default());
        }

        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: LitStr = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("expected only `name = \"...\"`"));
        }

        if key != "name" {
            return Err(syn::Error::new(
                key.span(),
                "supported options: `name = \"...\"`",
            ));
        }

        Ok(Self { name: Some(value) })
    }
}

fn formatted_function_source(function: &ItemFn) -> LitStr {
    formatted_item_source(Item::Fn(function.clone()))
}

fn formatted_struct_source(item: &ItemStruct) -> LitStr {
    formatted_item_source(Item::Struct(item.clone()))
}

fn formatted_enum_source(item: &ItemEnum) -> LitStr {
    formatted_item_source(Item::Enum(item.clone()))
}

fn formatted_item_source(item: Item) -> LitStr {
    let file = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: vec![item],
    };
    let source = prettyplease::unparse(&file);
    LitStr::new(&source, proc_macro2::Span::call_site())
}
