//! Trusted domain constructors for foreign types Creusot can't translate.
//!
//! `AccountHeader::new()` and `Reserved::ZERO` are foreign constructors Creusot
//! mis-handles; trusting these thin wrappers keeps them out of scope — the
//! bytes they produce (account header, reserved padding) are irrelevant to the
//! state invariants. This module also contains shared logical arithmetic used
//! by the per-type models in `nonzero` and other domain contracts.

#[cfg(creusot)]
use creusot_std::prelude::*;
use ticketing_interface::wire::{AccountHeader, AccountHeaderSpec, Reserved};

/// Mathematical minimum for unbounded integers used in Creusot contracts.
#[cfg(creusot)]
#[logic(open(crate), inline)]
pub(crate) fn int_min(left: Int, right: Int) -> Int {
    pearlite! { if left <= right { left } else { right } }
}

/// `AccountHeader::new()` is a contractless external constructor — Creusot
/// would give it an impossible precondition. Trust the wrapper; the header
/// bytes are irrelevant to the state invariants. Shared by every entity's
/// `new`.
#[cfg_attr(creusot, trusted)]
pub(crate) fn account_header<T: AccountHeaderSpec>() -> AccountHeader<T> {
    AccountHeader::new()
}

/// `Reserved::ZERO` is a tuple-struct const Creusot mis-translates (unnamed
/// field `.0`). Trust the wrapper; reserved padding is irrelevant to the
/// invariants. Shared by every entity's `new`.
#[cfg_attr(creusot, trusted)]
pub(crate) fn zero_reserved<const N: usize>() -> Reserved<N> {
    Reserved::ZERO
}
