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
//! *Compiler support: requires rustc 1.31+*
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
//! ## Acknowledgments
//!
//! The linker error technique is based on [Kixunil]'s crate [`dont_panic`].
//! Check out that crate for other convenient ways to require absence of panics.
//!
//! [Kixunil]: https://github.com/Kixunil
//! [`dont_panic`]: https://github.com/Kixunil/dont_panic

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::visit_mut::VisitMut;
use syn::{
    parse_macro_input, parse_quote, ArgCaptured, ArgSelf, ArgSelfRef, Attribute, ExprPath, FnArg,
    Ident, Item, ItemFn, Macro, ReturnType,
};

struct ReplaceSelf;

impl VisitMut for ReplaceSelf {
    fn visit_expr_path_mut(&mut self, i: &mut ExprPath) {
        if i.qself.is_none() && i.path.is_ident("self") {
            prepend_underscores_to_self(&mut i.path.segments[0].ident);
        }
    }

    fn visit_macro_mut(&mut self, i: &mut Macro) {
        // We can't tell in general whether `self` inside a macro invocation
        // refers to the self in the argument list or a different self
        // introduced within the macro. Heuristic: if the macro input contains
        // `fn`, then `self` is more likely to refer to something other than the
        // outer function's self argument.
        if !contains_fn(i.tts.clone()) {
            i.tts = fold_token_stream(i.tts.clone());
        }
    }

    fn visit_item_mut(&mut self, _i: &mut Item) {
        // Do nothing, as `self` now means something else.
    }
}

fn contains_fn(tts: TokenStream2) -> bool {
    tts.into_iter().any(|tt| match tt {
        TokenTree::Ident(ident) => ident == "fn",
        TokenTree::Group(group) => contains_fn(group.stream()),
        _ => false,
    })
}

fn fold_token_stream(tts: TokenStream2) -> TokenStream2 {
    tts.into_iter()
        .map(|tt| match tt {
            TokenTree::Ident(mut ident) => {
                prepend_underscores_to_self(&mut ident);
                TokenTree::Ident(ident)
            }
            TokenTree::Group(group) => {
                let content = fold_token_stream(group.stream());
                TokenTree::Group(Group::new(group.delimiter(), content))
            }
            other => other,
        })
        .collect()
}

fn prepend_underscores_to_self(ident: &mut Ident) {
    if ident == "self" {
        *ident = Ident::new("__self", Span::call_site());
    }
}

#[proc_macro_attribute]
pub fn no_panic(args: TokenStream, function: TokenStream) -> TokenStream {
    assert!(args.is_empty());

    let mut function = parse_macro_input!(function as ItemFn);

    let mut arg_ty = proc_macro2::TokenStream::new();
    let mut arg_pat = proc_macro2::TokenStream::new();
    let mut arg_val = proc_macro2::TokenStream::new();
    for (i, input) in function.decl.inputs.iter_mut().enumerate() {
        let numbered = Ident::new(&format!("__arg{}", i), Span::call_site());
        match input {
            FnArg::Captured(ArgCaptured {
                pat,
                colon_token,
                ty,
            }) => {
                arg_ty.extend(quote! {
                    #ty,
                });
                arg_pat.extend(quote! {
                    #pat #colon_token #ty,
                });
                arg_val.extend(quote! {
                    #numbered,
                });
                *pat = parse_quote!(#numbered);
            }
            FnArg::SelfRef(ArgSelfRef {
                and_token,
                lifetime,
                mutability,
                self_token,
            }) => {
                arg_ty.extend(quote! {
                    #and_token #lifetime #mutability Self,
                });
                arg_pat.extend(quote! {
                    __self: #and_token #lifetime #mutability Self,
                });
                arg_val.extend(quote! {
                    #self_token,
                });
                ReplaceSelf.visit_block_mut(&mut function.block);
            }
            FnArg::SelfValue(ArgSelf {
                mutability,
                self_token,
            }) => {
                arg_ty.extend(quote! {
                    Self,
                });
                arg_pat.extend(quote! {
                    #mutability __self: Self,
                });
                arg_val.extend(quote! {
                    #self_token,
                });
                *mutability = None;
                ReplaceSelf.visit_block_mut(&mut function.block);
            }
            _ => {}
        }
    }

    let has_inline = function
        .attrs
        .iter()
        .filter_map(Attribute::interpret_meta)
        .any(|meta| meta.name() == "inline");
    if !has_inline {
        function.attrs.push(parse_quote!(#[inline]));
    }

    let ret = match &function.decl.output {
        ReturnType::Default => quote!(-> ()),
        output @ ReturnType::Type(..) => quote!(#output),
    };
    let body = function.block;
    let message = format!(
        "\n\nERROR[no-panic]: detected panic in function `{}`\n",
        function.ident,
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
        let __result = (|#arg_pat| #ret #body as fn(#arg_ty) #ret)(#arg_val);
        core::mem::forget(__guard);
        __result
    }));

    TokenStream::from(quote!(#function))
}
