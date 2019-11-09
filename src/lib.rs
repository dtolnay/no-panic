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
//! <br>
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
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, parse_quote, Attribute, FnArg, Ident, ItemFn, PatType, ReturnType};

#[proc_macro_attribute]
pub fn no_panic(args: TokenStream, function: TokenStream) -> TokenStream {
    assert!(args.is_empty());

    let mut function = parse_macro_input!(function as ItemFn);

    let mut move_self = None;
    let mut arg_pat = Vec::new();
    let mut arg_val = Vec::new();
    for (i, input) in function.sig.inputs.iter_mut().enumerate() {
        let numbered = Ident::new(&format!("__arg{}", i), Span::call_site());
        match input {
            FnArg::Typed(PatType { pat, .. }) => {
                arg_pat.push(quote!(#pat));
                arg_val.push(quote!(#numbered));
                *pat = parse_quote!(mut #numbered);
            }
            FnArg::Receiver(_) => {
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
        .flat_map(Attribute::parse_meta)
        .any(|meta| meta.path().is_ident("inline"));
    if !has_inline {
        function.attrs.push(parse_quote!(#[inline]));
    }

    let ret = match &function.sig.output {
        ReturnType::Default => quote!(-> ()),
        output @ ReturnType::Type(..) => quote!(#output),
    };
    let body = function.block;
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
            #body
        })();
        core::mem::forget(__guard);
        __result
    }));

    TokenStream::from(quote!(#function))
}
