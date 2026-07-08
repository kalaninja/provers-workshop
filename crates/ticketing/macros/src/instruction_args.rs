//! Implementation of the [`InstructionArgs`](crate::derive_instruction_args)
//! derive: emits an `InstructionArgs` impl with a layout-derived
//! `TRAILING_RESERVED`.

use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput, Expr, ExprLit, Lit};

/// Expands the derive into an `InstructionArgs` impl, setting
/// `TRAILING_RESERVED` from the struct's trailing `Reserved<N>` pad (or `0`).
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "`InstructionArgs` can only be derived for structs",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "`InstructionArgs` cannot be derived for generic types",
        ));
    }

    let name = &input.ident;
    let trailing = match &data.fields {
        syn::Fields::Named(fields) => fields.named.last(),
        syn::Fields::Unit => None,
        syn::Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(
                &data.fields,
                "`InstructionArgs` requires a named-field or unit struct",
            ));
        },
    };

    let trait_path = quote! { crate::program::instruction::InstructionArgs };

    let Some((Some(field), len)) =
        trailing.and_then(|f| trailing_reserved_len(&f.ty).map(|len| (f.ident.as_ref(), len)))
    else {
        return Ok(quote! { impl #trait_path for #name {} });
    };

    let len_lit = Literal::usize_unsuffixed(len);
    Ok(quote! {
        impl #trait_path for #name {
            const TRAILING_RESERVED: usize = #len_lit;
        }

        // The trailing reserved field must occupy exactly the last
        // `TRAILING_RESERVED` bytes. `NoUninit` rules out trailing padding, so
        // this offset equality also pins the field's size — together they make
        // the short-payload parser unable to ever zero-fill a real field.
        const _: () = ::core::assert!(
            ::core::mem::offset_of!(#name, #field)
                == ::core::mem::size_of::<#name>() - #len_lit
        );
    })
}

/// Returns `Some(N)` when `ty` is a `Reserved<N>` with an integer-literal `N`.
fn trailing_reserved_len(ty: &syn::Type) -> Option<usize> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Reserved" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Const(Expr::Lit(ExprLit { lit: Lit::Int(value), .. })) => {
            value.base10_parse::<usize>().ok()
        },
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::{expand, trailing_reserved_len};

    #[test]
    fn trailing_reserved_len_reads_const_generic() {
        let reserved: syn::Type = parse_quote!(Reserved<3>);
        assert_eq!(trailing_reserved_len(&reserved), Some(3));
        let qualified: syn::Type = parse_quote!(wire::Reserved<1>);
        assert_eq!(trailing_reserved_len(&qualified), Some(1));
        let plain: syn::Type = parse_quote!(u64);
        assert_eq!(trailing_reserved_len(&plain), None);
    }

    #[test]
    fn instruction_args_auto_sets_trailing_reserved() {
        let input: syn::DeriveInput = parse_quote! {
            struct Padded {
                value: u16,
                _reserved: Reserved<2>,
            }
        };
        let out = expand(&input).expect("expands").to_string();
        assert!(out.contains("TRAILING_RESERVED"));
        assert!(out.contains("= 2"));
        assert!(out.contains("offset_of"));
    }

    #[test]
    fn instruction_args_defaults_to_zero_without_reserved() {
        let input: syn::DeriveInput = parse_quote! {
            struct Plain {
                value: u64,
            }
        };
        let out = expand(&input).expect("expands").to_string();
        assert!(out.contains("InstructionArgs for Plain"));
        assert!(!out.contains("TRAILING_RESERVED"));
    }

    #[test]
    fn instruction_args_rejects_enum() {
        let input: syn::DeriveInput = parse_quote!(
            enum NotAStruct {
                A,
            }
        );
        assert!(expand(&input).is_err());
    }
}
