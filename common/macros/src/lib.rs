use std::{collections::HashMap, env};

use proc_macro::{TokenStream};
use proc_macro2::{Span, Ident, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::{ExprLit, Data, DeriveInput, Expr, Fields, Lit, Meta, MetaList, MetaNameValue, PatLit, Path, Token, Type, Variant, parse::ParseStream, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::{Comma, Enum}};

use crate::helper::{absolute_path, gen_match_arm};

/// early exit with compile error
macro_rules! bail {
    ($text:expr) => {
        return ::syn::Error::new(::proc_macro2::Span::call_site(), $text).into_compile_error()
    };
    ($text:expr, ?2) => {
        return (::proc_macro2::TokenStream::new(), ::syn::Error::new(::proc_macro2::Span::call_site(), $text).into_compile_error())
    };
}

/// syntax:
/// ```ignore
/// #[derive(BoatImpl)]
/// enum Foo {
///     #[armanents(Set65)]
///     #[armanents(Avenger)]
///     #[armanents(Avenger)]  // sets `Foo::armanents` to hashmap { Weapon::Set65: 1, Weapon::Avenger: 2}
///     #[json("CustomJson")]  // implemeents the `FetchSprite` trait
///     #[level = 3]
///     Bar,
///     #[armanents(Brosok, 3)]  // equivalent to 3 `#[armanents(Brosok)]`
///     #[armanents(Avenger)]
///     #[level = 2]
///     Hi,
///     #[level = 1]
///     #[armanents(None)]
///     Olympias
/// }
/// ```
#[proc_macro_derive(
    BoatImpl,
    attributes(json, armanents, default_armanent, level, render_size)
)]
pub fn impl_boat_methods(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    TokenStream::from(impl_boat(ast))
}

fn impl_boat(ast: syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let generics = &ast.generics;

    let Data::Enum(data) = ast.data else {
        bail!("Only enums supported");
    };
    let variants = data.variants;

    // can't depend on common, therefore user should implement FetchSprite themselves
    let json = derive_json(&variants, name);
    let armanents = derive_armanents(&variants);
    let (assertions, level) = derive_level(&variants);
    
    quote! {
        #assertions
        impl #name #generics {
            #armanents
            #level
        }
        #json
    }
}

fn derive_json(variants: &Punctuated<Variant, Comma>, name: &Ident) -> TokenStream2 {
    let mut match_arms = vec![];

    for variant in variants {
        let ident = &variant.ident;
        let mut found_json = false;
        for attr in &variant.attrs {
            let Meta::List(MetaList { path, tokens, ..}) = &attr.meta else { continue };

            if path.is_ident("json") {
                // YES
                let match_arm = gen_match_arm(&variant.fields, ident, tokens);
                match_arms.push(match_arm);

                found_json = true;
                break;
            }
        }

        // if json(---) not found, use the enum variant's name
        if !found_json {
            let match_arm = gen_match_arm(&variant.fields, ident, ident.to_string());
            match_arms.push(match_arm);
        }
    }

    let trait_name = absolute_path("primitives::FetchSprite");

    // remember to add , after arm
    quote!(
        impl #trait_name for #name {
            fn fetch_sprite_str(&self) -> impl AsRef<str> {
                match self {
                    #(#match_arms),*
                }
            }
        }
    )
}

fn derive_armanents(variants: &Punctuated<Variant, Comma>) -> TokenStream2 {
    let mut match_arms = vec![];
    let (weapon_path, hashmap_path) = (absolute_path("Weapon"), absolute_path("hashmap"));

    for variant in variants {
        let ident = &variant.ident;
        let mut found_armanent = false;
        let mut no_weapon = false;

        let mut armanents: HashMap<String, u8> = HashMap::new();

        for attr in &variant.attrs {
            if !attr.path().is_ident("armanents") {
                continue;
            }
            let Meta::List(meta) = &attr.meta else { continue };

            let args = meta.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated).unwrap();
            let mut args = args.into_iter();

            if let Some(Expr::Path(weapon)) = args.next() {
                if weapon.path.is_ident("None") {
                    no_weapon = true;
                    break
                }
                let weapon_name = quote! {
                    #weapon_path::#weapon
                }.to_string();

                let count = if let Some(Expr::Lit(num)) = args.next() {
                    if let Lit::Int(num) = num.lit {
                        num.base10_parse::<u8>().unwrap()
                    } else {
                        bail!("Expected integer for num");
                    }
                } else {
                    // default to 1 weapon
                    1
                };

                *armanents.entry(weapon_name).or_insert(0) += count;
                
                found_armanent = true;
            }
        }
        
        if !found_armanent && !no_weapon {
            return syn::Error::new(Span::call_site(), "Specify armanents").into_compile_error();
        }

        if no_weapon && !armanents.is_empty() {
            bail!("Specified weapons when specifying None");
        }

        let construct = armanents.into_iter()
            .map(|(name, count)| {
                let weapon = name.parse::<TokenStream2>().unwrap();

                quote! {
                    #weapon => #count
                }
            });

        let ret = quote! {
            #hashmap_path! {
                #(#construct),*
            }
        };

        let match_arm = gen_match_arm(&variant.fields, ident, ret);
        match_arms.push(match_arm);
    }

    quote!(
        pub fn armanents(&self) -> ::std::collections::HashMap<#weapon_path, u8>{
            match self {
                #(#match_arms),*
            }
        }
    )
}

/// returns (assertions, impl code)
fn derive_level(variants: &Punctuated<Variant, Comma>) -> (TokenStream2, TokenStream2) {
    let mut match_arms = vec![];
    let level_path = absolute_path("primitives::Level");
    let mut const_asserts = vec![];

    for variant in variants {
        let mut found_level = false;
        let ident = &variant.ident;

        for attr in &variant.attrs {
            // note: this is not efficient for every derive func
            if !attr.path().is_ident("level") {
                continue;
            }
            if let Meta::NameValue(MetaNameValue { value, .. }) = &attr.meta
                && let Expr::Lit(ExprLit { lit, .. }) = value 
                && let Lit::Int(i) = lit
                && let Ok(num) = i.base10_parse::<u8>()
            {
                // TODO compile-time instead
                let arm = quote! {
                    Self::#ident => #level_path::try_from_u8(#num).expect("Should be caught at compile time")
                };
                match_arms.push(arm);

                let const_name = format!("_ASSERT_{}", ident);
                let const_name = Ident::new(&const_name, Span::call_site());
                let lhs: usize = (num - 1).into();
                const_asserts.push(quote! {
                    const #const_name: () = {
                        assert!(#lhs < #level_path::ALL.len(), "Level bigger than max")
                    };
                });
            } else {
                bail!("Use `level = <num>`", ?2);
            }
            found_level = true;
        }
        
        if !found_level {
            bail!("Must specify a level", ?2);
        }
    }

    let level_impl = quote! {
        #(#const_asserts)*
        pub fn level(&self) -> #level_path {
            match self {
                #(#match_arms),*
            }
        }
    };
    let const_assert = quote! {
        #(#const_asserts)*
    };

    (const_assert, level_impl)
}


mod helper {
    use proc_macro_crate::FoundCrate;
    use proc_macro2::TokenStream;
    use quote::{ToTokens, quote};
    use syn::{Fields, Path};

    /// doesn't care about data inside variant
    pub fn gen_match_arm(fields: &Fields, ident: &proc_macro2::Ident, match_ret: impl ToTokens) -> TokenStream {
        match fields {
            Fields::Unit => quote! {
                Self::#ident => #match_ret
            },
            Fields::Named(_) => quote! {
                Self::#ident { .. } => #match_ret
            },
            Fields::Unnamed(_) => quote! {
                Self::#ident(..) => #match_ret
            }
        }
    }
    // TODO replace with quote! { Self::#ident { .. } => #something }

    /// - `path` is relative to common root without the :: or `crate`
    pub fn absolute_path(path: &str) -> Path {
        let path = match proc_macro_crate::crate_name("common").unwrap() {
            FoundCrate::Itself => format!("crate::{path}"),
            FoundCrate::Name(name) => format!("{name}::{path}")
        };

        syn::parse_str(&path).unwrap()
    }
}