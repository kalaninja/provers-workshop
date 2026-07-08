//! Implementation of the
//! [`InstructionAccounts`](crate::derive_instruction_accounts) derive: turns
//! `#[codama(account(...))]` variant attributes into a typed accounts struct +
//! `parse` constructor per variant.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, ExprCall, ExprLit, ExprPath, Ident, Lit, Meta, Token,
    punctuated::Punctuated,
};

/// Expands the derive into one `<Variant>Accounts<'a>` struct + `parse` per
/// enum variant.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "`InstructionAccounts` can only be derived for enums",
        ));
    };

    let vis = &input.vis;
    let mut items = TokenStream2::new();

    for variant in &data.variants {
        let mut accounts = Vec::new();
        for attr in &variant.attrs {
            accounts.extend(codama_accounts_from_attr(attr)?);
        }

        if accounts.is_empty() {
            continue;
        }

        let variant_name = &variant.ident;
        let struct_name = format_ident!("{variant_name}Accounts");
        let count = accounts.len();
        let field_names: Vec<&Ident> = accounts.iter().map(|a| &a.name).collect();

        let signer_checks = accounts.iter().filter(|a| a.signer).map(|a| {
            let n = &a.name;
            quote! { crate::program::accounts::require_signer(#n)?; }
        });
        let writable_checks = accounts.iter().filter(|a| a.writable).map(|a| {
            let n = &a.name;
            quote! { crate::program::accounts::require_writable(#n)?; }
        });
        let identity_checks = accounts.iter().filter_map(|a| {
            let n = &a.name;
            a.identity.map(|id| {
                let (check, address) = id.tokens();
                quote! {
                    crate::program::accounts::#check(
                        #n,
                        &crate::program::accounts::#address,
                    )?;
                }
            })
        });

        let struct_doc = format!(
            "Typed accounts list for [`{variant_name}`], generated from its `#[codama(account(...))]` attributes."
        );
        let parse_doc = format!(
            "Extracts and validates the accounts for [`{variant_name}`] from the runtime account slice."
        );

        items.extend(quote! {
            #[doc = #struct_doc]
            #vis struct #struct_name<'a> {
                #( #[doc = "Account slot generated from `#[codama(account(name = ...))]`."]
                   pub #field_names: &'a mut ::pinocchio::AccountView, )*
            }

            impl<'a> #struct_name<'a> {
                #[doc = #parse_doc]
                ///
                /// # Errors
                ///
                /// Returns [`::pinocchio::error::ProgramError::NotEnoughAccountKeys`]
                /// or [`::pinocchio::error::ProgramError::InvalidArgument`] when the
                /// account count is wrong;
                /// [`::pinocchio::error::ProgramError::MissingRequiredSignature`] when
                /// a `signer`-flagged slot is not a signer;
                /// [`::pinocchio::error::ProgramError::InvalidAccountData`] when a
                /// `writable`-flagged slot is not writable; and
                /// [`::pinocchio::error::ProgramError::IncorrectProgramId`] /
                /// [`::pinocchio::error::ProgramError::UnsupportedSysvar`] when a slot
                /// with a fixed-address `default_value` is the wrong account.
                pub fn parse(
                    accounts: &'a mut [::pinocchio::AccountView],
                ) -> ::core::result::Result<Self, ::pinocchio::error::ProgramError> {
                    let [ #( #field_names ),* ] =
                        crate::program::accounts::exact_mut::<#count>(accounts)?;

                    #( #signer_checks )*
                    #( #writable_checks )*
                    #( #identity_checks )*

                    ::core::result::Result::Ok(Self { #( #field_names ),* })
                }
            }
        });
    }

    Ok(items)
}

/// A fixed-address identity check the derive emits from a `default_value`: a
/// generic `require_program`/`require_sysvar` call against a well-known address
/// constant — both live in `crate::program::accounts`, so adding a
/// program/sysvar is just a constant plus a `parse_identity` arm, no new
/// helper.
#[derive(Clone, Copy, Debug)]
enum Identity {
    /// `require_program` against the named `*_PROGRAM_ID` constant.
    Program(&'static str),
    /// `require_sysvar` against the named `*_SYSVAR_ID` constant.
    Sysvar(&'static str),
}

impl Identity {
    /// The `(check_fn, address_const)` idents to emit, both resolved against
    /// `crate::program::accounts`.
    fn tokens(self) -> (Ident, Ident) {
        match self {
            Identity::Program(address) => {
                (format_ident!("require_program"), format_ident!("{address}"))
            },
            Identity::Sysvar(address) => {
                (format_ident!("require_sysvar"), format_ident!("{address}"))
            },
        }
    }
}

struct AccountAttr {
    name: Ident,
    signer: bool,
    writable: bool,
    identity: Option<Identity>,
}

/// Parses every `account(...)` directive out of one `#[codama(...)]` attribute.
/// Non-`codama` attributes and non-`account` codama directives (e.g.
/// `discriminator`, `type`) yield an empty list; a malformed `account(...)`
/// errors.
fn codama_accounts_from_attr(attr: &syn::Attribute) -> syn::Result<Vec<AccountAttr>> {
    if !attr.path().is_ident("codama") {
        return Ok(Vec::new());
    }

    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
        .iter()
        .filter(|meta| meta.path().is_ident("account"))
        .map(parse_account_directive)
        .collect()
}

fn parse_account_directive(meta: &Meta) -> syn::Result<AccountAttr> {
    let Meta::List(list) = meta else {
        return Err(syn::Error::new_spanned(meta, "`account` must be `account(...)`"));
    };
    let entries = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

    let mut name: Option<Ident> = None;
    let mut signer = false;
    let mut writable = false;
    let mut identity = None;

    for entry in &entries {
        match entry {
            Meta::Path(path) if path.is_ident("signer") => signer = true,
            Meta::Path(path) if path.is_ident("writable") => writable = true,
            Meta::Path(path) if path.is_ident("optional") => {
                return Err(syn::Error::new_spanned(
                    path,
                    "`optional` accounts are not supported by `InstructionAccounts`: the generated \
                     `parse` requires an exact account count, so an optional slot would make the IDL \
                     (which marks it optional) and the runtime disagree. Remove `optional`, or add \
                     optional-account support to the derive first.",
                ));
            },
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = &nv.value else {
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "`name` value must be a string literal",
                    ));
                };
                let ident = syn::parse_str::<Ident>(&s.value()).map_err(|_| {
                    syn::Error::new_spanned(
                        s,
                        "`name` must be a valid Rust identifier (snake_case)",
                    )
                })?;
                name = Some(ident);
            },
            Meta::NameValue(nv) if nv.path.is_ident("default_value") => {
                identity = parse_identity(&nv.value)?;
            },
            // Ignore codama account keys the runtime does not act on
            // (`signer = "either"`, `docs`, future additions).
            _ => {},
        }
    }

    let name = name.ok_or_else(|| {
        syn::Error::new_spanned(meta, "`account(...)` requires `name = \"<snake_case_ident>\"`")
    })?;
    Ok(AccountAttr { name, signer, writable, identity })
}

/// Maps a `default_value` expression to a runtime identity check.
///
/// A `program(...)` / `sysvar(...)` default names a fixed on-chain address, so
/// it MUST resolve to a check: an unrecognized one (a `program("sytem")` typo,
/// or an unsupported program/sysvar) is a compile error rather than a silent
/// no-op that would drop the check while the IDL still advertises the default.
/// Every other `default_value` (`payer`, `pda(...)`, `account(...)`, ...) is
/// resolved by the handler / client, not asserted here, and yields `Ok(None)`.
fn parse_identity(expr: &Expr) -> syn::Result<Option<Identity>> {
    let Expr::Call(ExprCall { func, args, .. }) = expr else {
        return Ok(None);
    };
    let Expr::Path(ExprPath { path, .. }) = func.as_ref() else {
        return Ok(None);
    };
    let Some(func) = path.segments.last().map(|segment| segment.ident.to_string()) else {
        return Ok(None);
    };
    // Only program/sysvar defaults pin a fixed address that must be checked;
    // payer/pda/account and friends are resolved by the handler or client.
    if func != "program" && func != "sysvar" {
        return Ok(None);
    }
    let arg = args.first().and_then(|arg| match arg {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Some(s.value()),
        _ => None,
    });

    match (func.as_str(), arg.as_deref()) {
        ("program", Some("system")) => Ok(Some(Identity::Program("SYSTEM_PROGRAM_ID"))),
        ("program", Some("token")) => Ok(Some(Identity::Program("TOKEN_PROGRAM_ID"))),
        ("sysvar", Some("rent")) => Ok(Some(Identity::Sysvar("RENT_SYSVAR_ID"))),
        _ => Err(syn::Error::new_spanned(
            expr,
            "this `program(...)`/`sysvar(...)` default_value has no matching address constant in \
             `InstructionAccounts`; add a `*_PROGRAM_ID`/`*_SYSVAR_ID` constant and a \
             `parse_identity` arm, or use a `default_value` the derive does not assert \
             (e.g. `pda(...)`/`account(...)`)",
        )),
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::{Identity, codama_accounts_from_attr, expand, parse_identity};

    #[test]
    fn parses_full_account_directive() {
        let attr: syn::Attribute = parse_quote! {
            #[codama(account(name = "payer", signer, writable))]
        };
        let accounts = codama_accounts_from_attr(&attr).expect("parses");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name.to_string(), "payer");
        assert!(accounts[0].signer);
        assert!(accounts[0].writable);
        assert!(accounts[0].identity.is_none());
    }

    #[test]
    fn parses_minimal_account_directive() {
        let attr: syn::Attribute = parse_quote!(#[codama(account(name = "foo"))]);
        let accounts = codama_accounts_from_attr(&attr).expect("parses");
        assert_eq!(accounts[0].name.to_string(), "foo");
        assert!(!accounts[0].signer);
        assert!(!accounts[0].writable);
    }

    #[test]
    fn parses_program_and_sysvar_default_values() {
        let system: syn::Attribute = parse_quote! {
            #[codama(account(name = "system_program", default_value = program("system")))]
        };
        let token: syn::Attribute = parse_quote! {
            #[codama(account(name = "token_program", default_value = program("token")))]
        };
        let rent: syn::Attribute = parse_quote! {
            #[codama(account(name = "rent", default_value = sysvar("rent")))]
        };
        assert!(matches!(
            codama_accounts_from_attr(&system).unwrap()[0].identity,
            Some(Identity::Program("SYSTEM_PROGRAM_ID"))
        ));
        assert!(matches!(
            codama_accounts_from_attr(&token).unwrap()[0].identity,
            Some(Identity::Program("TOKEN_PROGRAM_ID"))
        ));
        assert!(matches!(
            codama_accounts_from_attr(&rent).unwrap()[0].identity,
            Some(Identity::Sysvar("RENT_SYSVAR_ID"))
        ));
    }

    #[test]
    fn payer_pda_and_account_default_values_produce_no_identity_check() {
        assert!(parse_identity(&parse_quote!(payer)).unwrap().is_none());
        assert!(parse_identity(&parse_quote!(pda("offering"))).unwrap().is_none());
        assert!(parse_identity(&parse_quote!(account("issuer"))).unwrap().is_none());
    }

    #[test]
    fn unrecognized_program_or_sysvar_default_value_is_rejected() {
        // A typo'd or unsupported program/sysvar must fail loudly, not silently
        // drop the identity check the IDL still advertises.
        assert!(parse_identity(&parse_quote!(program("sytem"))).is_err());
        assert!(parse_identity(&parse_quote!(program("associatedToken"))).is_err());
        assert!(parse_identity(&parse_quote!(sysvar("clock"))).is_err());
    }

    #[test]
    fn non_account_codama_directives_are_ignored() {
        let attr: syn::Attribute = parse_quote!(#[codama(discriminator(bytes = [1, 2]))]);
        assert!(codama_accounts_from_attr(&attr).expect("parses").is_empty());
        let other: syn::Attribute = parse_quote!(#[getset(skip)]);
        assert!(codama_accounts_from_attr(&other).expect("parses").is_empty());
    }

    #[test]
    fn rejects_missing_name() {
        let attr: syn::Attribute = parse_quote!(#[codama(account(signer, writable))]);
        assert!(codama_accounts_from_attr(&attr).is_err());
    }

    #[test]
    fn rejects_optional_account() {
        let attr: syn::Attribute = parse_quote!(#[codama(account(name = "maybe", optional))]);
        assert!(codama_accounts_from_attr(&attr).is_err());
    }

    #[test]
    fn rejects_non_identifier_name() {
        let attr: syn::Attribute = parse_quote!(#[codama(account(name = "123 bad ident"))]);
        assert!(codama_accounts_from_attr(&attr).is_err());
    }

    #[test]
    fn rejects_non_string_name_value() {
        let attr: syn::Attribute = parse_quote!(#[codama(account(name = 42))]);
        assert!(codama_accounts_from_attr(&attr).is_err());
    }

    #[test]
    fn expand_rejects_non_enum_input() {
        let input: syn::DeriveInput = parse_quote!(
            struct NotAnEnum;
        );
        assert!(expand(&input).is_err());
    }

    #[test]
    fn expand_emits_struct_impl_and_identity_checks_per_variant() {
        let input: syn::DeriveInput = parse_quote! {
            pub enum Fixture {
                #[codama(account(name = "payer", signer, writable))]
                #[codama(account(name = "system_program", default_value = program("system")))]
                A,
                #[codama(account(name = "buyer", signer, writable))]
                #[codama(account(name = "vault", writable))]
                B,
            }
        };
        let out = expand(&input).expect("expands").to_string();
        // Both per-variant structs are emitted.
        assert!(out.contains("struct AAccounts"));
        assert!(out.contains("struct BAccounts"));
        // The parse constructor exists for both.
        assert!(out.contains("impl < 'a > AAccounts < 'a >"));
        assert!(out.contains("impl < 'a > BAccounts < 'a >"));
        // The fixed-address default_value emits a generic require_program against
        // the matching address constant; absent one, no identity check.
        assert!(out.contains("require_program"));
        assert!(out.contains("SYSTEM_PROGRAM_ID"));
        assert!(!out.contains("TOKEN_PROGRAM_ID"));
    }
}
