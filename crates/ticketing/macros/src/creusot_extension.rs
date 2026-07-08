//! Implementation of the [`creusot_extension`](crate::creusot_extension)
//! attribute: generates an extension trait and its implementation from one
//! inherent-impl-shaped declaration.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, ImplItem, ImplItemFn, ItemImpl, Meta, Token, Visibility,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// Parsed `#[creusot_extension(trait Name)]` arguments.
pub(crate) struct Args {
    /// Name of the generated extension trait.
    pub(crate) trait_name: syn::Ident,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<Token![trait]>()?;
        Ok(Self { trait_name: input.parse()? })
    }
}

/// Parsed impl-shaped input.
pub(crate) struct ExtensionImpl {
    visibility: Visibility,
    item: ItemImpl,
}

impl Parse for ExtensionImpl {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attributes = Attribute::parse_outer(input)?;
        let visibility = input.parse::<Visibility>()?;
        let mut item = input.parse::<ItemImpl>()?;
        item.attrs.splice(0..0, attributes);
        Ok(Self { visibility, item })
    }
}

/// Expands an inherent-impl-shaped declaration into a trait and implementation.
pub(crate) fn expand(trait_name: &syn::Ident, input: ExtensionImpl) -> syn::Result<TokenStream2> {
    let ExtensionImpl { visibility, mut item } = input;
    if item.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            &item,
            "`creusot_extension` expects an inherent impl, not a trait impl",
        ));
    }
    if item.defaultness.is_some() {
        return Err(syn::Error::new_spanned(
            &item,
            "`creusot_extension` does not support default impls",
        ));
    }

    let trait_attrs = trait_attrs(&item.attrs);
    let impl_attrs = impl_attrs(&item.attrs);
    let generics = item.generics.clone();
    let trait_path = {
        let (_, ty_generics, _) = item.generics.split_for_impl();
        syn::parse_quote!(#trait_name #ty_generics)
    };

    let mut trait_methods = Vec::with_capacity(item.items.len());
    for impl_item in &mut item.items {
        let ImplItem::Fn(method) = impl_item else {
            return Err(syn::Error::new_spanned(
                impl_item,
                "`creusot_extension` supports methods only",
            ));
        };
        if !matches!(method.vis, Visibility::Inherited) {
            return Err(syn::Error::new_spanned(
                &method.vis,
                "`creusot_extension` methods must use inherited visibility",
            ));
        }

        trait_methods.push(trait_method(method));
        method.attrs = impl_method_attrs(&method.attrs);
    }

    item.attrs = impl_attrs;
    item.trait_ = Some((None, trait_path, Token![for](trait_name.span())));

    Ok(quote! {
        #(#trait_attrs)*
        #[allow(patterns_in_fns_without_body)]
        #visibility trait #trait_name #generics {
            #(#trait_methods)*
        }

        #item
    })
}

fn trait_method(method: &ImplItemFn) -> TokenStream2 {
    let attrs = trait_method_attrs(&method.attrs);
    let signature = &method.sig;
    quote! {
        #(#attrs)*
        #signature;
    }
}

fn trait_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| is_doc(attr) || (!is_impl_only_creusot_attr(attr) && !is_codegen_attr(attr)))
        .cloned()
        .collect()
}

fn impl_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !is_doc(attr)).cloned().collect()
}

fn trait_method_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter_map(|attr| {
            if is_doc(attr) {
                return Some(attr.clone());
            }
            if let Some(decl) = trait_logic_decl_attr(attr) {
                return Some(decl);
            }
            (!is_impl_only_creusot_attr(attr) && !is_codegen_attr(attr)).then(|| attr.clone())
        })
        .collect()
}

/// Creusot rejects `open`/`opaque` — a visibility for the *body* — on a
/// bodyless trait signature, so strip those tags from a `#[logic(...)]`
/// attribute copied to the trait declaration (the impl keeps the full form).
/// `None` for any attribute that needs no rewriting.
fn trait_logic_decl_attr(attr: &Attribute) -> Option<Attribute> {
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    if !list.path.is_ident("logic") {
        return None;
    }
    let tags = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).ok()?;
    let kept: Vec<&Meta> = tags
        .iter()
        .filter(|tag| !tag.path().is_ident("open") && !tag.path().is_ident("opaque"))
        .collect();
    if kept.len() == tags.len() {
        return None;
    }
    let mut decl = attr.clone();
    decl.meta = if kept.is_empty() {
        Meta::Path(list.path.clone())
    } else {
        syn::parse_quote!(logic(#(#kept),*))
    };
    Some(decl)
}

fn impl_method_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !is_doc(attr)).cloned().collect()
}

fn is_doc(attr: &Attribute) -> bool {
    attr.path().is_ident("doc")
}

fn is_codegen_attr(attr: &Attribute) -> bool {
    const CODEGEN_ATTRS: &[&str] = &[
        "cold",
        "export_name",
        "inline",
        "instruction_set",
        "link_section",
        "no_mangle",
        "optimize",
        "target_feature",
        "track_caller",
    ];

    if CODEGEN_ATTRS.iter().any(|name| attr.path().is_ident(name)) {
        return true;
    }

    let Meta::List(list) = &attr.meta else {
        return false;
    };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    let Ok(args) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
        return false;
    };
    args.iter().skip(1).any(|meta| CODEGEN_ATTRS.iter().any(|name| meta.path().is_ident(name)))
}

/// Creusot postconditions and proof-control attributes are meaningful on the
/// concrete implementation only. Preconditions are copied to both the trait
/// contract and the implementation so they cannot drift.
fn is_impl_only_creusot_attr(attr: &Attribute) -> bool {
    let Meta::List(list) = &attr.meta else {
        return false;
    };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }

    let Ok(args) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
        return false;
    };
    let mut args = args.iter();
    if !args.next().is_some_and(|meta| meta.path().is_ident("creusot")) {
        return false;
    }

    !args.any(|meta| meta.path().is_ident("requires"))
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use syn::parse_quote;

    use super::{Args, ExtensionImpl, expand};

    #[test]
    fn parses_explicit_trait_argument() {
        let args: Args = syn::parse_str("trait EntityOps").expect("attribute arguments parse");

        assert_eq!(args.trait_name, "EntityOps");
    }

    #[test]
    fn rejects_argument_without_trait_keyword() {
        assert!(syn::parse_str::<Args>("EntityOps").is_err());
    }

    #[test]
    fn rejects_trailing_argument_tokens() {
        assert!(syn::parse_str::<Args>("trait EntityOps extra").is_err());
    }

    fn expand_to_string(input: ExtensionImpl) -> String {
        expand(&format_ident!("EntityOps"), input).expect("extension impl expands").to_string()
    }

    #[test]
    fn generates_trait_and_impl_with_named_parameters() {
        let input: ExtensionImpl = parse_quote! {
            pub(crate) impl Entity {
                fn value(&self, expected: u64) -> bool {
                    self.value == expected
                }
            }
        };
        let output = expand_to_string(input);

        assert!(output.contains("pub (crate) trait EntityOps"));
        assert!(output.contains("fn value (& self , expected : u64) -> bool ;"));
        assert!(output.contains("impl EntityOps for Entity"));
    }

    #[test]
    fn copies_requires_but_keeps_ensures_on_impl() {
        let input: ExtensionImpl = parse_quote! {
            pub(crate) impl Entity {
                #[cfg_attr(creusot, requires(value > 0))]
                #[cfg_attr(creusot, ensures(result == value))]
                fn new(value: u64) -> Self
                where
                    Self: Sized,
                {
                    Self { value }
                }
            }
        };
        let output = expand_to_string(input);
        let trait_end = output.find("impl EntityOps for Entity").expect("impl emitted");
        let (trait_output, impl_output) = output.split_at(trait_end);

        assert!(trait_output.contains("requires"));
        assert!(!trait_output.contains("ensures"));
        assert!(impl_output.contains("requires"));
        assert!(impl_output.contains("ensures"));
        assert!(trait_output.contains("Self : Sized"));
    }

    #[test]
    fn keeps_docs_on_trait_and_lints_on_both_sides() {
        let input: ExtensionImpl = parse_quote! {
            /// Entity operations.
            #[allow(dead_code)]
            pub(crate) impl Entity {
                /// Returns the entity value.
                #[allow(clippy::unused_self)]
                #[inline]
                fn value(&self) -> u64 {
                    self.value
                }
            }
        };
        let output = expand_to_string(input);
        let trait_end = output.find("impl EntityOps for Entity").expect("impl emitted");
        let (trait_output, impl_output) = output.split_at(trait_end);

        assert!(trait_output.contains("Entity operations"));
        assert!(trait_output.contains("Returns the entity value"));
        assert!(!impl_output.contains("Entity operations"));
        assert!(!impl_output.contains("Returns the entity value"));
        assert!(trait_output.contains("allow"));
        assert!(impl_output.contains("allow"));
        assert!(!trait_output.contains("inline"));
        assert!(impl_output.contains("inline"));
    }

    #[test]
    fn preserves_impl_generics_on_trait_and_implementation() {
        let input: ExtensionImpl = parse_quote! {
            pub(crate) impl<T: Copy> Entity<T>
            where
                T: Default,
            {
                fn value(&self) -> T {
                    self.value
                }
            }
        };
        let output =
            expand(&format_ident!("EntityOps"), input).expect("generic extension impl expands");
        syn::parse2::<syn::File>(output.clone()).expect("expanded generic items parse");
        let output = output.to_string();

        assert!(output.contains("trait EntityOps < T : Copy >"));
        assert!(output.contains("impl < T : Copy > EntityOps < T > for Entity < T >"));
        assert!(output.contains("T : Default"));
    }

    #[test]
    fn rejects_associated_items_other_than_methods() {
        let input: ExtensionImpl = parse_quote! {
            pub(crate) impl Entity {
                const VALUE: u64 = 1;
            }
        };

        assert!(expand(&format_ident!("EntityOps"), input).is_err());
    }

    #[test]
    fn logic_visibility_moves_to_impl_only() {
        let input: ExtensionImpl = parse_quote! {
            pub(crate) impl Entity {
                #[cfg(creusot)]
                #[logic(open)]
                fn entity_invariant(self) -> bool {
                    self.value == 0
                }
            }
        };
        let output = expand_to_string(input);
        let trait_end = output.find("impl EntityOps for Entity").expect("impl emitted");
        let (trait_output, impl_output) = output.split_at(trait_end);

        // Trait declaration keeps `#[logic]` and the cfg gate, but not the
        // body-only `open` visibility (rejected on a bodyless signature).
        assert!(trait_output.contains("logic"));
        assert!(trait_output.contains("cfg (creusot)"));
        assert!(!trait_output.contains("open"));
        assert!(trait_output.contains("fn entity_invariant (self) -> bool ;"));
        // Impl keeps the full `#[logic(open)]` with the body.
        assert!(impl_output.contains("logic (open)"));
        assert!(impl_output.contains("cfg (creusot)"));
    }

    #[test]
    fn logic_strips_only_visibility_keeping_other_tags() {
        let input: ExtensionImpl = parse_quote! {
            pub(crate) impl Entity {
                #[cfg(creusot)]
                #[logic(open, inline)]
                fn is_zero(self) -> bool {
                    self.value == 0
                }
            }
        };
        let output = expand_to_string(input);
        let trait_end = output.find("impl EntityOps for Entity").expect("impl emitted");
        let (trait_output, impl_output) = output.split_at(trait_end);

        assert!(trait_output.contains("logic (inline)"));
        assert!(!trait_output.contains("open"));
        assert!(impl_output.contains("logic (open , inline)"));
    }
}
