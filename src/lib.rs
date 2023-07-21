//! [![github]](https://github.com/dtolnay/no-panic)&ensp;[![crates-io]](https://crates.io/crates/no-panic)&ensp;[![docs-rs]](https://docs.rs/no-panic)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! <br>
//!
//! A Rust attribute macro to require that the compiler prove a function can't
//! ever panic.
//!
//! ```toml
//! [dependencies]
//! no-panic = "0.1"
//! ```
//!
//! ```
//! use no_panic::no_panic;
//!
//! #[no_panic]
//! fn demo(s: &str) -> &str {
//!     &s[1..]
//! }
//!
//! fn main() {
//!     # fn demo(s: &str) -> &str {
//!     #     &s[1..]
//!     # }
//!     #
//!     println!("{}", demo("input string"));
//! }
//! ```
//!
//! If the function does panic (or the compiler fails to prove that the function
//! cannot panic), the program fails to compile with a linker error that
//! identifies the function name. Let's trigger that by passing a string that
//! cannot be sliced at the first byte:
//!
//! ```should_panic
//! # fn demo(s: &str) -> &str {
//! #     &s[1..]
//! # }
//! #
//! fn main() {
//!     println!("{}", demo("\u{1f980}input string"));
//! }
//! ```
//!
//! ```console
//!    Compiling no-panic-demo v0.0.1
//! error: linking with `cc` failed: exit code: 1
//!   |
//!   = note: /no-panic-demo/target/release/deps/no_panic_demo-7170785b672ae322.no_p
//! anic_demo1-cba7f4b666ccdbcbbf02b7348e5df1b2.rs.rcgu.o: In function `_$LT$no_pani
//! c_demo..demo..__NoPanic$u20$as$u20$core..ops..drop..Drop$GT$::drop::h72f8f423002
//! b8d9f':
//!           no_panic_demo1-cba7f4b666ccdbcbbf02b7348e5df1b2.rs:(.text._ZN72_$LT$no
//! _panic_demo..demo..__NoPanic$u20$as$u20$core..ops..drop..Drop$GT$4drop17h72f8f42
//! 3002b8d9fE+0x2): undefined reference to `
//!
//!           ERROR[no-panic]: detected panic in function `demo`
//!           '
//!           collect2: error: ld returned 1 exit status
//! ```
//!
//! The error is not stellar but notice the ERROR\[no-panic\] part at the end
//! that provides the name of the offending function.
//!
//! <br>
//!
//! ## Caveats
//!
//! - Functions that require some amount of optimization to prove that they do
//!   not panic may no longer compile in debug mode after being marked
//!   `#[no_panic]`.
//!
//! - Panic detection happens at link time across the entire dependency graph,
//!   so any Cargo commands that do not invoke a linker will not trigger panic
//!   detection. This includes `cargo build` of library crates and `cargo check`
//!   of binary and library crates.
//!
//! - The attribute is useless in code built with `panic = "abort"`.
//!
//! If you find that code requires optimization to pass `#[no_panic]`, either
//! make no-panic an optional dependency that you only enable in release builds,
//! or add a section like the following to Cargo.toml to enable very basic
//! optimization in debug builds.
//!
//! ```toml
//! [profile.dev]
//! opt-level = 1
//! ```
//!
//! If the code that you need to prove isn't panicking makes function calls to
//! non-generic non-inline functions from a different crate, you may need thin
//! LTO enabled for the linker to deduce those do not panic.
//!
//! ```toml
//! [profile.release]
//! lto = "thin"
//! ```
//!
//! If you want no_panic to just assume that some function you call doesn't
//! panic, and get Undefined Behavior if it does at runtime, see
//! [dtolnay/no-panic#16]; try wrapping that call in an `unsafe extern "C"`
//! wrapper.
//!
//! [dtolnay/no-panic#16]: https://github.com/dtolnay/no-panic/issues/16
//!
//! <br>
//!
//! ## Acknowledgments
//!
//! The linker error technique is based on [Kixunil]'s crate [`dont_panic`].
//! Check out that crate for other convenient ways to require absence of panics.
//!
//! [Kixunil]: https://github.com/Kixunil
//! [`dont_panic`]: https://github.com/Kixunil/dont_panic

#![doc(html_root_url = "https://docs.rs/no-panic/0.1.26")]
#![allow(
    clippy::doc_markdown,
    clippy::match_same_arms,
    clippy::missing_panics_doc
)]
#![cfg_attr(all(test, exhaustive), feature(non_exhaustive_omitted_patterns_lint))]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Error, Nothing, Result};
use syn::{
    parse_quote, FnArg, GenericArgument, Ident, ItemFn, Pat, PatType, Path, PathArguments,
    ReturnType, Token, Type, TypeInfer, TypeParamBound,
};

#[proc_macro_attribute]
pub fn no_panic(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let input = TokenStream2::from(input);
    let expanded = match parse(args, input.clone()) {
        Ok(function) => expand_no_panic(function),
        Err(parse_error) => {
            let compile_error = parse_error.to_compile_error();
            quote!(#compile_error #input)
        }
    };
    TokenStream::from(expanded)
}

fn parse(args: TokenStream2, input: TokenStream2) -> Result<ItemFn> {
    let function: ItemFn = syn::parse2(input)?;
    let _: Nothing = syn::parse2::<Nothing>(args)?;
    if function.sig.asyncness.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "no_panic attribute on async fn is not supported",
        ));
    }
    Ok(function)
}

// Convert `Path<impl Trait>` to `Path<_>`
fn make_impl_trait_wild(ret: &mut Type) {
    match ret {
        Type::ImplTrait(impl_trait) => {
            *ret = Type::Infer(TypeInfer {
                underscore_token: Token![_](impl_trait.impl_token.span),
            });
        }
        Type::Array(ret) => make_impl_trait_wild(&mut ret.elem),
        Type::Group(ret) => make_impl_trait_wild(&mut ret.elem),
        Type::Paren(ret) => make_impl_trait_wild(&mut ret.elem),
        Type::Path(ret) => make_impl_trait_wild_in_path(&mut ret.path),
        Type::Ptr(ret) => make_impl_trait_wild(&mut ret.elem),
        Type::Reference(ret) => make_impl_trait_wild(&mut ret.elem),
        Type::Slice(ret) => make_impl_trait_wild(&mut ret.elem),
        Type::TraitObject(ret) => {
            for bound in &mut ret.bounds {
                if let TypeParamBound::Trait(bound) = bound {
                    make_impl_trait_wild_in_path(&mut bound.path);
                }
            }
        }
        Type::Tuple(ret) => ret.elems.iter_mut().for_each(make_impl_trait_wild),
        Type::BareFn(_) | Type::Infer(_) | Type::Macro(_) | Type::Never(_) | Type::Verbatim(_) => {}
        #[cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
        _ => {}
    }
}

fn make_impl_trait_wild_in_path(path: &mut Path) {
    for segment in &mut path.segments {
        if let PathArguments::AngleBracketed(bracketed) = &mut segment.arguments {
            for arg in &mut bracketed.args {
                if let GenericArgument::Type(arg) = arg {
                    make_impl_trait_wild(arg);
                }
            }
        }
    }
}

fn expand_no_panic(mut function: ItemFn) -> TokenStream2 {
    let mut move_self = None;
    let mut arg_pat = Vec::new();
    let mut arg_val = Vec::new();
    for (i, input) in function.sig.inputs.iter_mut().enumerate() {
        let numbered = Ident::new(&format!("__arg{}", i), Span::call_site());
        match input {
            FnArg::Typed(PatType { pat, .. })
                if match pat.as_ref() {
                    Pat::Ident(pat) => pat.ident != "self",
                    _ => true,
                } =>
            {
                arg_pat.push(quote!(#pat));
                arg_val.push(quote!(#numbered));
                *pat = parse_quote!(mut #numbered);
            }
            FnArg::Typed(_) | FnArg::Receiver(_) => {
                move_self = Some(quote! {
                    if false {
                        loop {}
                        #[allow(unreachable_code)]
                        {
                            let __self = self;
                        }
                    }
                });
            }
        }
    }

    let has_inline = function
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("inline"));
    if !has_inline {
        function.attrs.push(parse_quote!(#[inline]));
    }

    let ret = match &function.sig.output {
        ReturnType::Default => quote!(-> ()),
        ReturnType::Type(arrow, output) => {
            let mut output = output.clone();
            make_impl_trait_wild(&mut output);
            quote!(#arrow #output)
        }
    };
    let stmts = function.block.stmts;
    let message = format!(
        "\n\nERROR[no-panic]: detected panic in function `{}`\n",
        function.sig.ident,
    );
    function.block = Box::new(parse_quote!({
        struct __NoPanic;
        extern "C" {
            #[link_name = #message]
            fn trigger() -> !;
        }
        impl core::ops::Drop for __NoPanic {
            fn drop(&mut self) {
                unsafe {
                    trigger();
                }
            }
        }
        let __guard = __NoPanic;
        let __result = (move || #ret {
            #move_self
            #(
                let #arg_pat = #arg_val;
            )*
            #(#stmts)*
        })();
        core::mem::forget(__guard);
        __result
    }));

    quote!(#function)
}
