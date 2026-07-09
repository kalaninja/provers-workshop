//! Concrete `is_set` for the foreign [`Address`], used by the `#[cfg(kani)]`
//! predicates.
//!
//! Under Creusot this is an opaque logical axiom; Kani needs a real definition,
//! so it is simply "not the all-zero default address". The whole module is
//! `#[cfg(kani)]` — the running code compares addresses with `==` directly.

use pinocchio::Address;

/// Predicate operations on an [`Address`].
pub(crate) trait AddressOps {
    /// Whether the address is set (not the all-zero default).
    fn is_set(self) -> bool;
}

impl AddressOps for Address {
    fn is_set(self) -> bool {
        self != Address::default()
    }
}
