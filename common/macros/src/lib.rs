mod helper;

use std::{collections::HashMap, ops::Not, sync::LazyLock};

use proc_macro::{TokenStream};
use proc_macro_crate::Error::CouldNotRead;
use proc_macro2::{Span, Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Expr, ExprLit, Lit, LitFloat, LitInt, Meta as SynMeta, MetaNameValue, Token, Variant, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma};
use helper::absolute_path;

use crate::helper::SpriteSheet;

/// early exit with compile error
/// 
/// note that the error messages aren't nice
macro_rules! bail {
    ($text:expr) => {
        return ::syn::Error::new(::proc_macro2::Span::call_site(), $text).into_compile_error()
    };
    ($text:expr, ?2) => {
        return (::proc_macro2::TokenStream::new(), ::syn::Error::new(::proc_macro2::Span::call_site(), $text).into_compile_error())
    };
    (?span = $span:expr, $text:expr) => {
        return ::syn::Error::new($span, $text).into_compile_error()
    }
}

const SPRITE_JSON: &str = include_str!("../../../client/assets/spritesheet.json");
static SHEET: LazyLock<SpriteSheet> = LazyLock::new(|| serde_json::from_str(SPRITE_JSON).unwrap());

/// syntax:
/// ```ignore
/// #[derive(BoatImpl)]
/// enum Foo {
///     #[armanents(Set65, default)]
///     #[armanents(Avenger)]
///     #[armanents(Avenger)]  // sets `Foo::armanents` to { Weapon::Set65: 1, Weapon::Avenger: 2 }
///     #[level = 3]
///     Bar,
///     #[armanents(Brosok, 3)]  // equivalent to 3 `#[armanents(Brosok)]`
///     #[armanents(Avenger, default)]
///     #[level = 2]
///     Hi,
///     #[level = 1]  // compiler error thrown if bigger than max
///     #[armanents(None)]
///     Olympias
/// }
/// ```
#[proc_macro_derive(
    BoatImpl,
    attributes(json, armanents, default_armanent, level)
)]
pub fn derive_boat_methods(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    TokenStream::from(impl_boat(ast))
}

/// syntax:
/// ```ignore
/// #[derive(FetchSprite)]
/// enum Bar {
///     Set65,  // "Set65"
///     Essex,  // "Essex"
///     #[json = "Akula")]
///     Cookies,  // "Akula"
///     Shell_100x1000Mmr,  // "Mark18"
///     // DoesNotExist  // compile error!
/// }
/// ```
/// 
/// if a cell with specified (or infered) name doesn't exist on the spritesheet, a compile error would be thrown
#[proc_macro_derive(
    FetchSprite,
    attributes(json)
)]
pub fn derive_fetch_sprite(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    TokenStream::from(impl_fetch_sprite(ast))
}
/// standard syntax:
/// #[length = int/float]  // infers from spritesheet
/// #[render_size(x, y)]   // directly generate `Size::render_size` in pixels
/// 
/// all numbers expect `render_size` are in meters
/// 
/// syntax for variants with "Shell_heightxlengthMmr" name, the macro would generate a size in meters of `length / 1000.0, width / 1000.0`
/// 
/// ### Example
/// ```ignore
/// #[derive(Size)]
/// enum Foo {
///     Shell_100x1000Mmr,  // 1, 0.1
///     #[length = 8]
///     Hq, // 8, 8 width fetched with ratio on sprite sheet
/// }
/// #[derive(Size)]
/// enum Bar {  // can't have both at same time right now due to how easy it is to break (TODO)
///     #[render_size(100, 80)]
///     Custom,                 // 100, 80 direct size
/// }
/// ```
#[proc_macro_derive(
    Size,
    attributes(length, render_size)
)]
pub fn derive_size(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let Data::Enum(DataEnum { variants, ..}) = ast.data else {
        panic!("Only enums supported");
    };
    let ident = &ast.ident;

    TokenStream::from(impl_size(&variants, ident))
}

/// syntax:
/// #[weapon_type = "AValidWeaponIdent"]
/// 
/// anything with variant starting with "Shell_" would be inferred shell
#[proc_macro_derive(WeaponType, attributes(weapon_type))]
pub fn derive_weapon_type(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let Data::Enum(DataEnum {variants, .. }) = ast.data else {
        panic!("Only enums supported");
    };
    let ident = ast.ident;

    TokenStream::from(derive_weapon_type_inner(variants, ident))
}

/// in knots or "None" for 0.0
#[proc_macro_derive(MaxSpeed, attributes(max_speed))]
pub fn derive_max_speed(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let Data::Enum(DataEnum { variants, .. }) = ast.data else {
        panic!("Only enums supported");
    };
    let ident = ast.ident;

    TokenStream::from(derive_max_speed_inner(variants, ident))
}
fn impl_boat(ast: DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let generics = &ast.generics;

    let Data::Enum(data) = ast.data else {
        bail!("Only enums supported");
    };
    let variants = data.variants;

    let armanents = derive_armanents(&variants);
    let (assertions, level) = derive_level(&variants);
    
    quote! {
        #assertions
        impl #name #generics {
            #armanents
            #level
        }
    }
}

/// implement the FetchSprite trait either with default or with overwritten
fn impl_fetch_sprite(ast: DeriveInput) -> TokenStream2 {
    let name = &ast.ident;

    let Data::Enum(data) = ast.data else {
        bail!("Only enums supported");
    };
    let variants = data.variants;

    let mut match_arms = vec![];

    for variant in variants {
        let ident = &variant.ident;

        if ident.to_string().starts_with("Shell_") {
            match_arms.push(quote! {
                Self::#ident => "Mark18"
            });
            continue;
        }

        if let Some(value) = variant.attrs.iter().find_map(|attr| {
            if let SynMeta::NameValue(MetaNameValue { path, value, .. }) = &attr.meta
                && path.is_ident("json")
            {
                Some(value)
            } else {
                None
            }
        }) {
            let path = if let Expr::Lit(lit) = value
                && let Lit::Str(path) = &lit.lit
            {
                path.value()
            } else {
                bail!("Expected string")
            };
            if !SHEET.contains(&path) {
                bail!(format!("Specified JSON name {path:?} does not exist in the file, {SHEET:?}"));
            }
            let match_arm = quote! {
                Self::#ident => #path
            };
            match_arms.push(match_arm);
        } else {
            let stringified = ident.to_string();
            if !SHEET.contains(&stringified) {
                bail!(format!("Default name {stringified} does not exist in the file, {SHEET:?}"))
            }
            let match_arm = quote! {
                Self::#ident => #stringified
            };
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
    let mut default_weapon_arms = vec![];
    let (weapon_path, hashmap_path, weapon_data_path) = (absolute_path("Weapon"), absolute_path("util::OrderedHashMap"), absolute_path("primitives::WeaponData"));

    for variant in variants {
        let ident = &variant.ident;
        let mut no_weapon = false;
        let mut default_weapon = None;
        let mut armanents: Vec<(String, u16)> = Vec::new();

        for args in variant.attrs.iter()
            .filter(|attr| attr.path().is_ident("armanents"))
            .map(|attr| {
                attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
            })
        {
            let Ok(args) = args else { bail!("Expected expression(s) seperated by Comma") };
            let mut args = args.into_iter();

            let Some(Expr::Path(weapon)) = args.next() else { bail!("Empty/non-path weapon attribute") };
            if weapon.path.is_ident("None") {
                no_weapon = true;
                break
            }
            let weapon_name = quote! {
                #weapon_path::#weapon
            }.to_string();  // required by hashmap for Eq

            let second_arg = args.next();
            let count = if let Some(Expr::Lit(num)) = &second_arg {
                if let Lit::Int(num) = &num.lit {
                    num.base10_parse::<u16>().expect("Too many weapons")
                } else {
                    bail!("Expected integer for num");
                }
            } else {
                // default to 1 weapon
                1
            };

            // equivalent to *armanents.entry(weapon_name.clone()).or_insert(0) += count;
            if let Some((_name, counter)) = armanents.iter_mut().find(|(name, _)| *name == weapon_name) {
                *counter += count;
            } else {
                armanents.push((weapon_name.clone(), count));
            }
            
            if let Some(Expr::Path(path)) = second_arg
                && path.path.is_ident("default")
            {
                if default_weapon.is_some() {
                    bail!("Multiple default armanents")
                }
                default_weapon = Some(weapon_name)
            } else if let Some(Expr::Path(path)) = args.next()
                && path.path.is_ident("default")
            {
                if default_weapon.is_some() {
                    bail!("Multiple default armanents")
                }
                default_weapon = Some(weapon_name)
            }
        }

        if let Some(default) = default_weapon {
            let default = default.parse::<TokenStream2>().unwrap();
            default_weapon_arms.push(quote! {
                Self::#ident => ::std::option::Option::Some(#default)
            });
        } else if armanents.is_empty() {
            default_weapon_arms.push(quote! {
                Self::#ident => ::std::option::Option::None
            });
        } else {
            bail!("Didn't specify default weapon even though variant has armanents")
        }
        if armanents.is_empty() && !no_weapon {
            bail!("Specify armanents");
        }

        if no_weapon && !armanents.is_empty() {
            bail!("Specified weapons when specifying None");
        }

        let construct = armanents.into_iter()
            .map(|(name, count)| {
                let weapon = name.parse::<TokenStream2>().unwrap();

                quote! {
                    (#weapon, #weapon_data_path {
                        max: #count,
                        avaliable: #count
                    })
                }
            });

        let ret = quote! {
            #hashmap_path::from_arr([
                #(#construct),*
            ])
        };

        let match_arm = quote! {
            Self::#ident => #ret
        };
        match_arms.push(match_arm);
    }

    quote!(
        pub fn armanents(&self) -> #hashmap_path<#weapon_path, #weapon_data_path>{
            match self {
                #(#match_arms),*
            }
        }
        pub fn default_weapon(&self) -> ::std::option::Option<#weapon_path> {
            match self {
                #(#default_weapon_arms),*
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
        let ident = &variant.ident;

        if let Some(attr) = variant.attrs.iter().find(|attr| attr.path().is_ident("level"))
            && let SynMeta::NameValue(MetaNameValue { value, .. }) = &attr.meta
            && let Expr::Lit(ExprLit { lit, .. }) = value 
            && let Lit::Int(i) = lit
            && let Ok(num) = i.base10_parse::<u8>()
        {
            let arm = quote! {
                Self::#ident => #level_path::try_from_u8(#num).expect("Should be caught at compile time")
            };
            match_arms.push(arm);

            let const_name = format!("_ASSERT_{}", ident);
            let const_name = Ident::new(&const_name, Span::call_site());
            let lhs: usize = (num - 1).into();
            const_asserts.push(quote! {
                const #const_name: () = assert!(#lhs < <#level_path as ::strum::EnumCount>::COUNT, "Level bigger than max");
            });
        } else {
            bail!("must specify a `level = <num>`", ?2);
        }
    }

    let level_impl = quote! {
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

/// derives the Size trait
fn impl_size(variants: &Punctuated<Variant, Comma>, name: &Ident) -> TokenStream2 {
    let size_trait = absolute_path("primitives::Size");
    let mut match_arms: Vec<TokenStream2> = vec![];
    let mut render_size_arms: Vec<TokenStream2> = vec![];

    for variant in variants {
        let ident = &variant.ident;
        if ident.to_string().starts_with("Shell_") {
            let bail_format = "Expected format: Shell_heightxlengthMmr";
            let mut name = ident.to_string();
            let mut name = name.split_off("Shell_".len());
            for mmr in "Mmr".chars().rev() {
                if name.pop() != Some(mmr) {
                    bail!(bail_format)
                }
            }
            let mut name = name.split('x');
            
            let height: f32 = name.next().expect(bail_format).parse::<u16>().expect("Expected height to be u16").into();
            let length: f32 = name.next().expect(bail_format).parse::<u16>().expect("Expected width to be u16").into();
            let height = height / 1000.0;
            let length = length / 1000.0;

            assert_eq!(name.next(), None);

            match_arms.push(quote! {
                Self::#ident => ::std::convert::Into::into((#length, #height))
            });
            continue;
        }
        if let Some(attr) = variant.attrs.iter()
            .find(|attr| attr.path().is_ident("length"))
        {
            let length = {
                if let SynMeta::NameValue(MetaNameValue { value, .. }) = &attr.meta
                    && let Expr::Lit(ExprLit { lit, .. }) = value
                {
                    if let Lit::Float(float) = lit
                    && let Ok(float) = float.base10_parse::<f32>()
                    {
                        float
                    } else if let Lit::Int(int) = lit
                        && let Ok(int) = int.base10_parse::<u16>()
                    {
                        int.into()
                    } else {
                        bail!("Expected integer/float")
                    }
                } else {
                    bail!("Expected #[length = num]")
                }
            };

            let raw_size = if let Some(size) = SHEET.get_size(&ident.to_string()) {
                size
            } else {
                if let Some(attr) = variant.attrs.iter()
                    .find(|attr| attr.path().is_ident("json"))
                    && let SynMeta::NameValue(MetaNameValue { value, .. }) = &attr.meta
                    && let Expr::Lit(expr) = value
                    && let Lit::Str(json_name) = &expr.lit
                    && let json_name = json_name.value()
                {
                    SHEET.get_size(&json_name).expect("Unreachable, should be caught in derive_json")
                } else {
                    bail!(format!("{ident:?} doesn't exist in sheet with no custom json impl"));
                }
            };
            
            // using the raw dimension's aspect ratio to find out the width in meters
            // length x alpha = raw_size.x
            // height x alpha = raw_size.y
            let alpha = raw_size.w as f32 / length;
            let height = raw_size.h as f32 / alpha;

            let match_arm = quote! {
                Self::#ident => ::std::convert::Into::into((#length, #height))
            };

            match_arms.push(match_arm);
        } else if let Some(attr) = variant.attrs.iter()
            .find(|attr| attr.path().is_ident("render_size"))
        {
            // TODO maybe support ommiting one side
            let (length, height) = if let Ok(list) = attr.parse_args_with(Punctuated::<LitFloat, Token![,]>::parse_terminated)
                && let mut args = list.into_iter()
                && let Some(first) = args.next()
                && let Some(second) = args.next()
                && let Ok(length) = first.base10_parse::<f32>()
                && let Ok(height) = second.base10_parse::<f32>()
            {
                (length, height)
            } else if let Ok(list) = attr.parse_args_with(Punctuated::<LitInt, Token![,]>::parse_terminated)
                && let mut args = list.into_iter()
                && let Some(first) = args.next()
                && let Some(second) = args.next()
                && let Ok(length) = first.base10_parse::<u16>()
                && let Ok(height) = second.base10_parse::<u16>()
            {
                (length.into(), height.into())
            } else {
                bail!("Expected #[render_size(a, b)]")
            };
            render_size_arms.push(quote! {
                Self::#ident => ::std::convert::Into::into((#length, #height))
            });
        } else {
            bail!("Expected #[render_size(a, v)] or #[length = x]")
        }
    }

    if render_size_arms.is_empty().not() && !match_arms.is_empty().not() {
        bail!("Can only be one implementation for this trait")
    }

    if !match_arms.is_empty() {
        quote! {
            impl #size_trait for #name {
                fn size(&self) -> ::bevy::prelude::Vec2 {
                    match self {
                        #(#match_arms),*
                    }
                }
            }
        }
    } else if !render_size_arms.is_empty() {
        quote! {
            impl #size_trait for #name {
                fn size(&self) -> ::bevy::prelude::Vec2 {
                    unimplemented("Derive did not specify size in pixels")
                }
                fn render_size(&self) -> ::bevy::prelude::Vec2 {
                    match self {
                        #(#render_size_arms),*
                    }
                }
            }
        }
    } else {
        unreachable!()
    }
}

fn derive_weapon_type_inner(variants: Punctuated<Variant, Comma>, ident: Ident) -> TokenStream2 {
    let mut match_arms = vec![];
    let type_path = absolute_path("WeaponType");

    for variant in variants {
        let ident = variant.ident;
        if ident.to_string().starts_with("Shell_") {
            match_arms.push(quote! {
                Self::#ident => #type_path::Shell
            });
            continue;
        }
        if let Some(path) = variant.attrs.iter().find_map(|attr| {
            if let SynMeta::NameValue(MetaNameValue { path, value, ..}) = &attr.meta
                && path.is_ident("weapon_type")
                && let Expr::Lit(expr) = value
                && let Lit::Str(weapon_type) = &expr.lit
            {
                Some(weapon_type.parse::<Ident>().unwrap())
            } else {
                None
            }
        }) {
            match_arms.push(quote! {
                Self::#ident => #type_path::#path
            });
        } else {
            return syn::Error::new(ident.span(), "Expected weapon type attribute").into_compile_error()
        }
    }

    quote! {
        impl #ident {
            pub fn weapon_type(&self) -> #type_path {
                match self {
                    #(#match_arms),*
                }
            }
        }
    }
}

fn derive_max_speed_inner(variants: Punctuated<Variant, Comma>, ident: Ident) -> TokenStream2 {
    let mut match_arms = vec![];
    let speed_path = absolute_path("primitives::Speed");

    for variant in variants {
        let ident = variant.ident;

        if let Some(attr) = variant.attrs.iter().find(|attr| attr.path().is_ident("max_speed"))
            && let SynMeta::NameValue(MetaNameValue { value, ..}) = &attr.meta
            && let Expr::Lit(ExprLit { lit, .. }) = value
        {
            let max_speed: f32 = if let Lit::Float(float) = lit {
                float.base10_parse::<f32>().expect("f32 err")
            } else if let Lit::Int(int) = lit {
                int.base10_parse::<u16>().expect("u16 overflow").into()
            } else if let Lit::Str(string) = lit
                && string.value() == "None"
            {
                0.0  // difficult to do cross-enum validation here
            } else {
                bail!(?span = attr.span(), "Expected int/float/None");
            };

            match_arms.push(quote! {
                Self::#ident => #max_speed
            });
        } else {
            bail!(?span = ident.span(), "Expected #[max_speed = x] attribute")
        }
    }

    quote! {
        impl #ident {
            /// 0 speed represents no speed
            pub fn max_speed(&self) -> #speed_path {
                #speed_path::from_knots(match self {
                    #(#match_arms),*
                })
            }
        }
    }
}
