#![cfg_attr(not(check_cfg), allow(unexpected_cfgs))]
#![allow(
    clippy::doc_markdown,
    clippy::match_same_arms,
    clippy::missing_panics_doc,
    clippy::uninlined_format_args
)]
#![cfg_attr(all(test, exhaustive), feature(non_exhaustive_omitted_patterns_lint))]

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Error, Nothing, Result};
use syn::{
    parse_quote, FnArg, GenericArgument, Ident, ItemFn, Pat, PatType, Path, PathArguments,
    ReturnType, Token, Type, TypeInfer, TypeParamBound,
};

#[proc_macro_attribute]
pub fn abort_on_panic(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let input = TokenStream2::from(input);
    let expanded = match parse(args, input.clone()) {
        Ok(function) => expand_abort_on_panic(function),
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
            "abort_on_panic attribute on async fn is not supported",
        ));
    }
    Ok(function)
}

// Convert `Path<impl Trait>` to `Path<_>`
fn make_impl_trait_wild(ret: &mut Type) {
    match ret {
        #![cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
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

fn expand_abort_on_panic(mut function: ItemFn) -> TokenStream2 {
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

    let ret = match &function.sig.output {
        ReturnType::Default => quote!(-> ()),
        ReturnType::Type(arrow, output) => {
            let mut output = output.clone();
            make_impl_trait_wild(&mut output);
            quote!(#arrow #output)
        }
    };
    let stmts = function.block.stmts;
    function.block = Box::new(parse_quote!({
        let __guard = noexcept::__private::AbortOnDrop;
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
