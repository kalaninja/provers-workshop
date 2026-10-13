//! Flux refinement vocabulary and the lane's trusted foreign-type boundary.
//!
//! Two layers, split by build visibility:
//!
//! - the verification-only surface - the `flux_rs::defs!` logic block and the
//!   `#[extern_spec]` registrations that refine the foreign wire types - is
//!   `#[cfg(flux)]`-gated and vanishes from ordinary/SBF builds;
//! - the executable accessors, setters, constructors, and numeric bridges ship
//!   in every build; their `#[flux_rs::sig]` clauses erase outside `cargo
//!   flux`.
//!
//! Modeling choices and their trusted-surface cost:
//!
//! - `Event`/`Ticket` use opaque models to avoid expanding recursive wire
//!   headers and padding into the refinement types. Constructors and field
//!   access remain trusted. Event frames include opaque identities for the
//!   header, name, and reserved bytes; those identities do not verify the
//!   helper bodies or their correspondence to account bytes.
//! - The standard-library specifications supply no numeric `NonZero` model at
//!   this pin. Our opaque model indexes `NonZero<T>` by an integer value;
//!   trusted `nz_get`/`nz_from` preserve that value across the `u64` boundary.
//!   The exact sale quantity is checked using this assumed correspondence.
//!   Naming the sealed `ZeroablePrimitive` bound needs `nonzero_internals`,
//!   enabled only for verification.
//! - Standard-library refinements for `Result`/`Option` and checked arithmetic
//!   are loaded by `just flux verify` with `-Fstd-extern-specs=on`. Arithmetic
//!   checks use strict overflow mode and no lane-owned arithmetic axioms.
use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{Event, EventStatus};

// --- verification-only refinement surface (erased from ordinary/SBF builds)
// ------------------

#[cfg(flux)]
flux_rs::defs! {
    fn int_min(a: int, b: int) -> int {
        if a <= b { a } else { b }
    }

    // Uninterpreted "address is set" (not the all-zero default) - the analog of
    // Verus's opaque `is_set`.
    fn is_set(a: int) -> bool;

    // Legal event status step on the `closed` bool: `open -> closed`, or unchanged.
    fn legal_status(from: bool, to: bool) -> bool {
        from == to || (!from && to)
    }

    // Frame: every field except `sold` unchanged.
    fn same_except_sold(old: Event, new: Event) -> bool {
        new.organizer == old.organizer
            && new.created_slot == old.created_slot
            && new.updated_slot == old.updated_slot
            && new.capacity == old.capacity
            && new.price == old.price
            && new.name == old.name
            && new.header == old.header
            && new.reserved == old.reserved
            && new.bump == old.bump
            && new.closed == old.closed
    }

    // Frame: every field except `closed` unchanged.
    fn same_except_status(old: Event, new: Event) -> bool {
        new.organizer == old.organizer
            && new.created_slot == old.created_slot
            && new.updated_slot == old.updated_slot
            && new.capacity == old.capacity
            && new.price == old.price
            && new.name == old.name
            && new.header == old.header
            && new.reserved == old.reserved
            && new.sold == old.sold
            && new.bump == old.bump
    }
}

/// Opaque `Address` refined by a ghost identity; equality is id-equality and
/// `is_set` is the uninterpreted predicate over it (mirrors the Creusot / Verus
/// opaque `Address`).
#[cfg(flux)]
#[flux_rs::extern_spec(solana_address)]
#[refined_by(id: int)]
#[opaque]
struct Address;

#[cfg(flux)]
#[flux_rs::extern_spec(solana_address)]
impl PartialEq for Address {
    #[flux_rs::sig(fn(&Address[@x], &Address[@y]) -> bool[x == y])]
    fn eq(&self, other: &Address) -> bool;
}

/// An opaque identity for the complete bounded string, including its unused
/// bytes.
#[cfg(flux)]
#[flux_rs::extern_spec(ticketing_interface::wire)]
#[refined_by(id: int)]
#[opaque]
struct BoundedString<const N: usize>;

/// `Event`, opaque, refined by every field. Header, name, and reserved bytes
/// use opaque identities; their correspondence to Rust values is trusted.
/// The invariants ride on the type, so `event_invariant` never needs threading
/// through a `requires` clause - every `Event` value carries it by
/// construction.
#[cfg(flux)]
#[flux_rs::extern_spec(ticketing_interface)]
#[refined_by(
    organizer: int,
    created_slot: int,
    updated_slot: int,
    capacity: int,
    price: int,
    sold: int,
    name: int,
    header: int,
    reserved: int,
    bump: int,
    closed: bool,
)]
#[invariant(is_set(organizer))]
#[invariant(0 < capacity)]
#[invariant(0 < price)]
#[invariant(0 <= sold)]
#[invariant(sold <= capacity)]
#[invariant(capacity * price <= u64::MAX)]
#[opaque]
struct Event;

/// `Ticket`, opaque, refined by its purchase fields.
#[cfg(flux)]
#[flux_rs::extern_spec(ticketing_interface)]
#[refined_by(
    event: int,
    buyer: int,
    quantity: int,
    paid_lamports: int,
    bump: int,
    created_slot: int,
    updated_slot: int,
)]
#[invariant(is_set(event))]
#[invariant(is_set(buyer))]
#[opaque]
struct Ticket;

/// Opaque numeric model; the trusted `u64` bridges below preserve its value.
#[cfg(flux)]
#[flux_rs::extern_spec(core::num)]
#[refined_by(value: int)]
#[opaque]
struct NonZero<T: core::num::ZeroablePrimitive>;

// --- trusted exec shims: bodies ship, `#[flux_rs::sig]` clauses erase off-flux
// --------------

/// `NonZeroU64` -> the same positive numeric value. This correspondence with
/// the opaque model is trusted; callers use it to prove exact quantities.
#[flux_rs::trusted]
#[flux_rs::sig(fn(x: NonZeroU64[@n]) -> u64{v: v == n && v > 0})]
pub fn nz_get(x: NonZeroU64) -> u64 {
    x.get()
}

/// A positive `u64` -> `NonZeroU64`. The `None` arm is dead under the `v > 0`
/// precondition (no panic, no `unwrap`).
#[flux_rs::trusted]
#[flux_rs::sig(fn(v: u64[@n]) -> NonZeroU64[n] requires n > 0)]
pub fn nz_from(v: u64) -> NonZeroU64 {
    match NonZeroU64::new(v) {
        Some(n) => n,
        None => NonZeroU64::MIN,
    }
}

/// Whether `a * b` fits in `u64` - the overflow guard for `event_create`.
/// Checked against the standard-library `checked_mul` and `Option` models.
#[flux_rs::sig(fn(a: u64, b: u64) -> bool[a * b <= u64::MAX])]
pub fn mul_fits(a: u64, b: u64) -> bool {
    a.checked_mul(b).is_some()
}

/// Trusted `Event` capacity accessor.
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &Event[@ev]) -> u64[ev.capacity])]
pub fn get_capacity(e: &Event) -> u64 {
    e.capacity.get()
}

/// Trusted `Event` unit-price accessor.
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &Event[@ev]) -> u64[ev.price])]
pub fn get_price(e: &Event) -> u64 {
    e.price_lamports.get()
}

/// Trusted `Event` sold-counter accessor.
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &Event[@ev]) -> u64[ev.sold])]
pub fn get_sold(e: &Event) -> u64 {
    e.sold
}

/// Trusted "sales are open" accessor.
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &Event[@ev]) -> bool[!ev.closed])]
pub fn is_open(e: &Event) -> bool {
    matches!(e.status, EventStatus::SalesOpen)
}

/// Trusted organizer-equality accessor.
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &Event[@ev], who: &Address[@o]) -> bool[ev.organizer == o])]
pub fn organizer_eq(e: &Event, who: &Address) -> bool {
    e.organizer == *who
}

/// Trusted sold-counter update; only `sold` changes and it must stay within
/// capacity (so the type invariant is preserved).
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &mut Event[@ev], v: u64{v <= ev.capacity})
    ensures e: Event{new: same_except_sold(ev, new) && new.sold == v})]
pub fn set_sold(e: &mut Event, v: u64) {
    e.sold = v;
}

/// Trusted status update: `open -> closed`; only `status` changes.
#[flux_rs::trusted]
#[flux_rs::sig(fn(e: &mut Event[@ev])
    ensures e: Event{new: same_except_status(ev, new) && new.closed})]
pub fn set_status_closed(e: &mut Event) {
    e.status = EventStatus::SalesClosed;
}
