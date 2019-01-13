//! A Rust attribute macro to require that the compiler prove a function can't ever
//! panic.
//!
//! ```toml
//! [dependencies]
//! no-panic = "0.1"
//! ```
//!
//! ```rust
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
use syn::visit_mut::VisitMut;
use syn::{
    ArgCaptured, ArgSelfRef, Attribute, ExprPath, FnArg, Ident, Item, ItemFn, Pat, PatIdent,
};

fn mangled_marker_name(original: &Ident) -> String {
    let len = original.to_string().len();
    format!("_Z22RUST_PANIC_IN_FUNCTIONI{}{}E", len, original)
}

struct ReplaceSelf;

impl VisitMut for ReplaceSelf {
    fn visit_expr_path_mut(&mut self, i: &mut ExprPath) {
        if i.qself.is_none()
            && i.path.leading_colon.is_none()
            && i.path.segments.len() == 1
            && i.path.segments[0].ident == "self"
            && i.path.segments[0].arguments.is_empty()
        {
            i.path.segments[0].ident = Ident::new("__self", Span::call_site());
        }
    }

    fn visit_item_mut(&mut self, _i: &mut Item) {
        /* do nothing, as `self` now means something else */
    }
}

#[proc_macro_attribute]
pub fn no_panic(args: TokenStream, function: TokenStream) -> TokenStream {
    assert!(args.is_empty());

    let mut function = parse_macro_input!(function as ItemFn);

    let mut arg_pat = Vec::new();
    let mut arg_val = Vec::new();
    for input in &mut function.decl.inputs {
        match input {
            FnArg::Captured(ArgCaptured {
                pat: Pat::Ident(pat @ PatIdent { subpat: None, .. }),
                ..
            }) => {
                let ident = &pat.ident;
                arg_pat.push(quote!(#pat));
                arg_val.push(quote!(#ident));
                if pat.by_ref.is_none() {
                    pat.mutability = None;
                }
                pat.by_ref = None;
            }
            FnArg::SelfRef(ArgSelfRef {
                mutability: Some(_),
                self_token,
                ..
            }) => {
                arg_pat.push(quote!(__self));
                arg_val.push(quote!(#self_token));
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

    let body = function.block;
    let ident = Ident::new(&mangled_marker_name(&function.ident), Span::call_site());
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
                let #arg_pat = #arg_val;
            )*
            #body
        })();
        core::mem::forget(__guard);
        __result
    }));

    TokenStream::from(quote!(#function))
}
