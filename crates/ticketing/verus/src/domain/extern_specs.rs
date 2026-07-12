//! Verus external specifications for the foreign types the cores touch.
//!
//! Two layers, split by build visibility:
//!
//! - the *spec-only* surface (type registrations and `spec fn`s) sits in a
//!   `#[cfg(feature = "verus")] verus!{}` block and vanishes from ordinary/SBF
//!   builds;
//! - the trusted *exec* wrappers live in an ungated `verus!{}` block whose
//!   ghost clauses erase outside `cargo verus`, leaving plain-Rust bodies the
//!   mutators/cores call at runtime.
//!
//! `NonZeroU64` itself is modeled by `vstd::std_specs::nonzero` (registration,
//! the `@` view, `new`/`get` specs); this file only bridges what vstd does not
//! cover: `checked_mul` and `Ord::min` on `NonZero` (trusted wrappers, the
//! Creusot `ax_*` analog), the opaque `Address`, and the wire/state type
//! registrations.
use core::num::NonZeroU64;

// Types used only by the feature-gated spec surface below.
#[cfg(feature = "verus")]
use pinocchio::{Address, error::ProgramError};
#[cfg(feature = "verus")]
use ticketing_interface::wire::BoundedString;
use ticketing_interface::wire::{AccountHeader, AccountHeaderSpec, Reserved};
#[cfg(feature = "verus")]
use ticketing_interface::{Event, EventStatus, Ticket};
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::math::min;
#[cfg(feature = "verus")]
use vstd::prelude::*;

// --- spec-only surface: erased from ordinary/SBF builds
// -------------------------------------
#[cfg(feature = "verus")]
verus! {

/// Opaque `Address` (analog of Creusot's `domain/address.rs`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAddress(Address);

/// Uninterpreted logical predicate: the address is set (not the all-zero default).
pub uninterp spec fn is_set(a: Address) -> bool;

/// Runtime `Address` equality, related to spec equality on the opaque value.
pub assume_specification[ <Address as PartialEq>::eq ](a: &Address, b: &Address) -> (r: bool)
    ensures
        r == (*a == *b),
;

// Wire padding + error + state type registrations. `AccountHeader<T>` needs
// `accept_recursive_types` because its `PhantomData<fn() -> T>` makes `Event` recurse through a
// (phantom) function-pointer position; it is opaque, so that is sound.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(T)]
pub struct ExAccountHeader<T>(AccountHeader<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExReserved<const N: usize>(Reserved<N>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExBoundedString<const N: usize>(BoundedString<N>);

/// `ProgramError` registered transparently (a plain fieldless-plus-`Custom(u32)` enum), so exec
/// code constructs error variants directly - no trusted constructors needed.
#[verifier::external_type_specification]
pub struct ExProgramError(ProgramError);

#[verifier::external_type_specification]
pub struct ExEventStatus(EventStatus);

#[verifier::external_type_specification]
pub struct ExEvent(Event);

#[verifier::external_type_specification]
pub struct ExTicket(Ticket);

} // verus!
// --- trusted exec wrappers: ghost clauses erase, bodies ship
// ------------------------------
verus! {

/// `checked_mul`: `None` exactly on `u64` overflow, else the product on the
/// `@` view (Creusot's `ax_checked_mul`). `NonZero`'s `checked_mul` is a
/// per-type inherent method `vstd` does not spec.
#[verifier::external_body]
pub fn nz_checked_mul(a: NonZeroU64, b: NonZeroU64) -> (r: Option<NonZeroU64>)
    ensures
        (a@ * b@ > u64::MAX) ==> r is None,
        (a@ * b@ <= u64::MAX) ==> (r matches Some(n) && n@ == a@ * b@),
{
    a.checked_mul(b)
}

/// `Ord::min` on the `@` view (Creusot's `ax_min`). `vstd` specs `Ord` methods
/// through `OrdSpec`, which it does not implement for `NonZero`.
#[verifier::external_body]
pub fn nz_min(a: NonZeroU64, b: NonZeroU64) -> (r: NonZeroU64)
    ensures
        r@ as int == min(a@ as int, b@ as int),
{
    a.min(b)
}

/// Trusted account-header constructor (the header bytes are irrelevant to the
/// state invariants).
#[verifier::external_body]
#[verifier::allow(undeclared_external_trait)]
pub fn account_header<T: AccountHeaderSpec>() -> AccountHeader<T> {
    AccountHeader::new()
}

/// Trusted all-zero reserved-padding constructor (irrelevant to the state
/// invariants).
#[verifier::external_body]
pub fn zero_reserved<const N: usize>() -> Reserved<N> {
    Reserved::ZERO
}

} // verus!
