//! Trusted, Creusot-modeled domain operations for the foreign `NonZeroU64`.
//!
//! `creusot-std` has no model for `NonZero`, so the runtime operations are
//! routed through these `#[trusted]` methods whose contracts relate them to the
//! opaque logical value `int` — the axioms (assumed, not proven) the rest of
//! the proof rests on. In a normal (non-Creusot) build they are plain
//! pass-throughs.

use core::num::NonZeroU64;

#[cfg(creusot)]
use creusot_std::prelude::*;
use ticketing_macros::creusot_extension;

#[cfg(creusot)]
use crate::domain::axioms::int_min;

#[creusot_extension(trait NonZeroOps)]
/// Axiom-level (trusted, Creusot-modeled) operations on a [`NonZeroU64`].
pub(crate) impl NonZeroU64 {
    /// Uninterpreted logical (unbounded-integer) value, `>= 1`.
    #[cfg(creusot)]
    #[trusted]
    #[logic(opaque)]
    #[ensures(result >= 1)]
    fn int(self) -> Int {
        dead
    }

    /// `NonZeroU64::new`: `Some` iff the input is non-zero, carrying its value.
    #[cfg_attr(creusot, trusted)]
    #[cfg_attr(creusot, ensures((v@ == 0) == (result == None)))]
    #[cfg_attr(creusot, ensures(forall<x: NonZeroU64> result == Some(x) ==> x.int() == v@))]
    fn ax_new(v: u64) -> Option<NonZeroU64> {
        NonZeroU64::new(v)
    }

    /// `get`, with its `u64` result tied to the logical value `int`.
    #[cfg_attr(creusot, trusted)]
    #[cfg_attr(creusot, ensures(result@ == self.int()))]
    fn ax_get(self) -> u64 {
        self.get()
    }

    /// `Ord::min`, tied to `int`.
    #[cfg_attr(creusot, trusted)]
    #[cfg_attr(creusot, ensures(result.int() == int_min(self.int(), other.int())))]
    fn ax_min(self, other: NonZeroU64) -> NonZeroU64 {
        self.min(other)
    }

    /// `checked_mul`: `Some(self * other)` tied to `int` when it fits in
    /// `u64`, `None` exactly on overflow.
    #[cfg_attr(creusot, trusted)]
    #[cfg_attr(creusot, ensures((result == None) == (self.int() * other.int() > u64::MAX@)))]
    #[cfg_attr(creusot, ensures(forall<r: NonZeroU64> result == Some(r) ==> r.int() == self.int() * other.int()))]
    fn ax_checked_mul(self, other: NonZeroU64) -> Option<NonZeroU64> {
        self.checked_mul(other)
    }
}
