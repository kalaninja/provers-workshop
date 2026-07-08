//! Implementation of the [`StateAccount`](crate::derive_state_account)
//! derive: emits an explicit marker-trait implementation for a concrete
//! account-state struct.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput};

/// Expands the derive into an empty `StateAccount` implementation.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    if !matches!(&input.data, Data::Struct(_)) {
        return Err(syn::Error::new_spanned(
            input,
            "`StateAccount` can only be derived for structs",
        ));
    }
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "`StateAccount` cannot be derived for generic types",
        ));
    }

    let name = &input.ident;
    let trait_path = quote! { crate::program::state::StateAccount };

    Ok(quote! { impl #trait_path for #name {} })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::expand;

    #[test]
    fn state_account_emits_marker_impl() {
        let input: syn::DeriveInput = parse_quote! {
            struct AccountState {
                value: u64,
            }
        };

        let output = expand(&input).expect("expands").to_string();

        assert!(output.contains("StateAccount for AccountState"));
    }

    #[test]
    fn state_account_rejects_enum() {
        let input: syn::DeriveInput = parse_quote! {
            enum NotAnAccount {
                Variant,
            }
        };

        assert!(expand(&input).is_err());
    }

    #[test]
    fn state_account_rejects_generic_struct() {
        let input: syn::DeriveInput = parse_quote! {
            struct GenericAccount<T> {
                value: T,
            }
        };

        assert!(expand(&input).is_err());
    }
}
