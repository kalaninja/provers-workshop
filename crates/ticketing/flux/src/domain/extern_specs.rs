//! Flux refinement vocabulary + the trusted shims for what Flux does not model
//! at this rev.
//!
//! Two layers, split by build visibility:
//!
//! - the verification-only surface - the `flux_rs::defs!` logic block and the
//!   `#[extern_spec]` registrations that refine the foreign wire types - is
//!   `#[cfg(flux)]`-gated and vanishes from ordinary/SBF builds;
//! - the trusted exec shims (the `Event`/`Ticket` field accessors, the
//!   `NonZeroU64` <-> `u64` bridges, and the one nonlinear `mul_le_mono` lemma)
//!   ship in every build; their `#[flux_rs::sig]` clauses erase outside `cargo
//!   flux`.
//!
//! Modeling choices forced by Flux at this rev (each an honest trusted-surface
//! cost, larger than Verus's):
//!
//! - `Event`/`Ticket` are **opaque** (the `core::alloc::Layout` pattern): the
//!   state invariant rides on the type via `#[invariant]`, but Flux cannot see
//!   through a foreign struct whose fields include padding (`Reserved<N>`) and
//!   a self-referential header (`AccountHeader<Event>`, a recursive sort). So
//!   the cores read/write state through the trusted accessors below.
//! - `NonZeroU64` is **not** modeled. Its `core::num::NonZero<T>` carrier is
//!   generic over the sealed, unstable `ZeroablePrimitive`, and Flux cannot
//!   refine a generic type parameter by an `int` value; so `NonZero` values are
//!   bridged to refined `u64` through the trusted `nz_get`/`nz_from` shims and
//!   all accounting arithmetic runs on `u64` (where Flux is precise). The cost:
//!   the exact `min(requested, remaining)` quantity is not surfaced (only that
//!   the sale is clamped within remaining capacity and priced without
//!   overflow).
//! - Flux's `Result`/`Option` refinements are not visible to a downstream crate
//!   here, so the cores refine their `Ok` payload type rather than the `Result`
//!   discriminant; "on error, unchanged" and the exact `Ok <=> cond`
//!   characterization are dropped.
use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{Event, EventStatus};

// --- verification-only refinement surface (erased from ordinary/SBF builds)
// ------------------

#[cfg(flux)]
flux_rs::defs! {
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

/// `Event`, opaque, refined by its accounting-relevant fields. The three
/// invariants ride on the type, so `event_invariant` never needs threading
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

/// `Ticket`, opaque, refined by the addresses its invariant constrains.
#[cfg(flux)]
#[flux_rs::extern_spec(ticketing_interface)]
#[refined_by(event: int, buyer: int)]
#[invariant(is_set(event))]
#[invariant(is_set(buyer))]
#[opaque]
struct Ticket;

// --- trusted exec shims: bodies ship, `#[flux_rs::sig]` clauses erase off-flux
// --------------

/// `NonZeroU64` -> its `u64` value (always positive). The bridge into the
/// refined `u64` world where Flux reasons precisely.
#[flux_rs::trusted]
#[flux_rs::sig(fn(x: NonZeroU64) -> u64{v: v > 0})]
pub fn nz_get(x: NonZeroU64) -> u64 {
    x.get()
}

/// A positive `u64` -> `NonZeroU64`. The `None` arm is dead under the `v > 0`
/// precondition (no panic, no `unwrap`).
#[flux_rs::trusted]
#[flux_rs::sig(fn(v: u64{v > 0}) -> NonZeroU64)]
pub fn nz_from(v: u64) -> NonZeroU64 {
    match NonZeroU64::new(v) {
        Some(n) => n,
        None => NonZeroU64::MIN,
    }
}

/// The one nonlinear fact this lane needs: monotonicity of `*`. From `a <= b`
/// and `b * c <= u64::MAX`, conclude `a * c <= u64::MAX`. Verus discharges the
/// analog with `by(nonlinear_arith)`; here it is a trusted lemma - the honest
/// nonlinear entry in the trusted-surface column. A no-op at runtime.
#[flux_rs::trusted]
#[flux_rs::sig(fn(a: u64, b: u64, c: u64) requires a <= b && b * c <= u64::MAX ensures a * c <= u64::MAX)]
pub fn mul_le_mono(_a: u64, _b: u64, _c: u64) {}

/// Positivity of `*`: a product of positives is positive (so `quantity * price`
/// yields a `NonZeroU64` payment). Trusted, like [`mul_le_mono`]; a no-op at
/// runtime.
#[flux_rs::trusted]
#[flux_rs::sig(fn(a: u64, b: u64) requires 0 < a && 0 < b ensures 0 < a * b)]
pub fn mul_pos(_a: u64, _b: u64) {}

/// Whether `a * b` fits in `u64` - the overflow guard for `event_create`. Flux
/// does not carry `u64::checked_mul`'s spec into this crate, so the check that
/// relates the boolean result to the `a * b <= u64::MAX` bound is trusted (the
/// bound the event invariant then carries for the account's whole life).
#[flux_rs::trusted]
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
