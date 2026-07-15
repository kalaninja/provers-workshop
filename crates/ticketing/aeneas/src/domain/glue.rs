//! The opaque trusted surface: the only code that touches `Event`/`Ticket`/
//! `NonZeroU64` fields.
//!
//! Charon is told to treat this module as opaque (`[package.metadata.charon]
//! opaque`), so in the extracted model each of these is an uninterpreted
//! function over the opaque wire types, and the Lean proofs give them trusted
//! `@[step]` specs (accessors relate to a logical `EventModel`; the setters
//! give frame + update; the constructors give their postconditions). This is
//! the analog of Flux's `#[extern_spec]` registrations and Creusot's
//! `#[trusted]` shims - the declared trusted computing base.
//!
//! The bodies are ordinary Rust and ship in the real program (host + SBF); only
//! their *extraction* is abstracted. They stay minimal - the accounting logic
//! that reasons about them lives in [`crate::domain::event`] (transparent).
use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{
    Event, EventName, EventStatus, Ticket,
    wire::{AccountHeader, Reserved},
};

/// `NonZeroU64` -> its `u64` value (always positive).
pub fn nz_get(x: NonZeroU64) -> u64 {
    x.get()
}

/// A positive `u64` -> `NonZeroU64`. The `None` arm is dead under the callers'
/// `v > 0` obligation (no panic, no `unwrap`).
pub fn nz_from(v: u64) -> NonZeroU64 {
    match NonZeroU64::new(v) {
        Some(n) => n,
        None => NonZeroU64::MIN,
    }
}

/// `Event` capacity accessor.
pub fn get_capacity(e: &Event) -> u64 {
    e.capacity.get()
}

/// `Event` unit-price accessor.
pub fn get_price(e: &Event) -> u64 {
    e.price_lamports.get()
}

/// `Event` sold-counter accessor.
pub fn get_sold(e: &Event) -> u64 {
    e.sold
}

/// "Sales are open" accessor.
pub fn is_open(e: &Event) -> bool {
    matches!(e.status, EventStatus::SalesOpen)
}

/// Organizer-equality accessor.
pub fn organizer_eq(e: &Event, who: &Address) -> bool {
    e.organizer == *who
}

/// Sold-counter update; callers keep `v <= capacity`, so the event invariant is
/// preserved. Only `sold` changes.
pub fn set_sold(e: &mut Event, v: u64) {
    e.sold = v;
}

/// Status update: `open -> closed`. Only `status` changes.
pub fn set_status_closed(e: &mut Event) {
    e.status = EventStatus::SalesClosed;
}

/// Builds a fresh sales-open event with a zeroed counter - the base case for
/// the event invariant. Capacity and price arrive as positive `u64` (the world
/// the accounting runs in) and are re-materialized as `NonZeroU64` here at the
/// wire boundary.
pub fn event_new(
    organizer: Address,
    capacity: u64,
    price_lamports: u64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> Event {
    Event {
        header: AccountHeader::new(),
        created_slot,
        updated_slot: created_slot,
        organizer,
        capacity: nz_from(capacity),
        price_lamports: nz_from(price_lamports),
        sold: 0,
        name,
        bump,
        status: EventStatus::SalesOpen,
        _reserved: Reserved::ZERO,
    }
}

/// Builds a buyer's ticket for a priced sale.
pub fn ticket_new(
    event: Address,
    buyer: Address,
    paid_lamports: NonZeroU64,
    quantity: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> Ticket {
    Ticket {
        header: AccountHeader::new(),
        created_slot,
        updated_slot: created_slot,
        event,
        buyer,
        quantity,
        paid_lamports,
        bump,
        _reserved: Reserved::ZERO,
    }
}
