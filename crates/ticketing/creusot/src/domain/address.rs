//! Trusted, Creusot-modeled domain operations for the foreign [`Address`].
//!
//! `Address` comes from `solana-address`, which Creusot cannot see into, so
//! the runtime operations are routed through `#[trusted]` methods whose
//! contracts relate them to the opaque logical predicate `is_set` — the
//! axioms (assumed, not proven) the rest of the proof rests on. In a normal
//! (non-Creusot) build they are plain pass-throughs.

#[cfg(creusot)]
use creusot_std::prelude::*;
use pinocchio::Address;
use ticketing_macros::creusot_extension;

#[creusot_extension(trait AddressOps)]
/// Axiom-level (trusted, Creusot-modeled) operations on an [`Address`].
pub(crate) impl Address {
    /// Uninterpreted logical predicate: is set (not the all-zero default).
    #[cfg(creusot)]
    #[logic(opaque)]
    fn is_set(self) -> bool {
        dead
    }

    /// `*self == *other`; trusted (foreign type has no model).
    #[cfg_attr(creusot, trusted)]
    #[cfg_attr(creusot, ensures(result == (*self == *other)))]
    fn ax_eq(&self, other: &Address) -> bool {
        *self == *other
    }
}
