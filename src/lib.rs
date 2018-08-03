#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{Ident, ItemFn};

fn mangled_marker_name(original: &Ident) -> String {
    let len = original.to_string().len();
    format!("_Z22RUST_PANIC_IN_FUNCTIONI{}{}E", len, original)
}

#[proc_macro_attribute]
pub fn no_panic(args: TokenStream, function: TokenStream) -> TokenStream {
    assert!(args.is_empty());

    let mut function: ItemFn = syn::parse(function).unwrap();
    let ident = Ident::new(&mangled_marker_name(&function.ident), Span::call_site());
    let body = function.block;
    function.block = Box::new(parse_quote!({
        extern crate core;
        struct __NoPanic;
        extern "C" {
            fn #ident() -> !;
        }
        impl core::ops::Drop for __NoPanic {
            fn drop(&mut self) {
                unsafe {
                    #ident();
                }
            }
        }
        let __guard = __NoPanic;
        let __result = #body;
        core::mem::forget(__guard);
        __result
    }));

    TokenStream::from(quote!(#function))
}
