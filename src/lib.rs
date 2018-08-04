//! A Rust attribute macro to require that the compiler prove a function can't ever
//! panic.
//!
//! ```toml
//! [dependencies]
//! no-panic = "0.1"
//! ```
//!
//! ```rust
//! #![feature(use_extern_macros)]
//!
//! extern crate no_panic;
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
//! cannot panic), the program fails to compile with a linker error that identifies
//! the function name. Let's trigger that by passing a string that cannot be sliced
//! at the first byte:
//!
//! ```rust,should_panic
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
//! 3002b8d9fE+0x2): undefined reference to `RUST_PANIC_IN_FUNCTION<demo>'
//!           collect2: error: ld returned 1 exit status
//! ```
//!
//! The error is not stellar but notice the useful part at the end that provides the
//! name of the offending function: ```undefined reference to
//! `RUST_PANIC_IN_FUNCTION<demo>'```
//!
//! *Requires rustc \>=1.30.0.*
//!
//! ## Caveats
//!
//! - Functions that require some amount of optimization to prove that they do not
//!   panic may no longer compile in debug mode after being marked `#[no_panic]`.
//!
//! - The attribute is useless in code built with `panic = "abort"`.
//!
//! ## Acknowledgments
//!
//! The linker error technique is based on [**@Kixunil**]'s crate [`dont_panic`].
//! Check out that crate for other convenient ways to require absence of panics.
//!
//! [**@Kixunil**]: https://github.com/Kixunil
//! [`dont_panic`]: https://github.com/Kixunil/dont_panic

#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{FnArg, FnDecl, Ident, ItemFn, Pat};

fn mangled_marker_name(original: &Ident) -> String {
    let len = original.to_string().len();
    format!("_Z22RUST_PANIC_IN_FUNCTIONI{}{}E", len, original)
}

fn captured_args(decl: &FnDecl) -> impl Iterator<Item = &Ident> {
    decl.inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Captured(captured) => Some(&captured.pat),
            _ => None,
        }).filter_map(|pat| match pat {
            Pat::Ident(pat) => Some(&pat.ident),
            _ => None,
        })
}

#[proc_macro_attribute]
pub fn no_panic(args: TokenStream, function: TokenStream) -> TokenStream {
    assert!(args.is_empty());

    let mut function: ItemFn = syn::parse(function).unwrap();
    let ident = Ident::new(&mangled_marker_name(&function.ident), Span::call_site());
    let arg_pat = captured_args(&function.decl);
    let arg_value = captured_args(&function.decl);
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
        let __result = (move || {
            #(
                let #arg_pat = #arg_value;
            )*
            #body
        })();
        core::mem::forget(__guard);
        __result
    }));

    TokenStream::from(quote!(#function))
}
