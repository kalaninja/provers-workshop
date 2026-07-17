//! The trusted surface: the only code that touches `Event`/`Ticket`/
//! `NonZeroU64` fields.
//!
//! hax cannot translate these bodies (they read `NonZeroU64` and the wire
//! structs' padded, self-referential layout), so each carries a
//! `#[hax_lib::lean::replace(...)]` giving its trusted Lean model over the
//! logical `Event`/`Ticket` structures declared in the committed model preamble
//! (`verif/ticketing_hax/model_preamble.lean`, spliced in by
//! `scripts/hax/verify.sh`). This is the analog of Aeneas's opaque
//! `domain::glue` + `@[step]` specs, Flux's `#[extern_spec]` registrations, and
//! Creusot's `#[trusted]` shims - the declared trusted computing base, here as
//! concrete Lean definitions the proofs unfold.
//!
//! The Rust bodies are ordinary and ship in the real program (host + SBF); only
//! their *extraction* is replaced. The accounting logic that reasons about them
//! lives in [`crate::domain::event`] (transparent, proved).
use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{
    Event, EventName, EventStatus, Ticket,
    wire::{AccountHeader, Reserved},
};

/// `NonZeroU64` -> its `u64` value (always positive).
#[cfg_attr(
    hax,
    hax_lib::lean::replace(
        "@[spec] def nz_get (x : core_models.num.nonzero.NonZero u64) : RustM u64 :=
  pure (core_models.num.nonzero.nzVal x)"
    )
)]
pub fn nz_get(x: NonZeroU64) -> u64 {
    x.get()
}

/// A positive `u64` -> `NonZeroU64`. The `None` arm is dead under the callers'
/// `v > 0` obligation (no panic, no `unwrap`).
#[cfg_attr(
    hax,
    hax_lib::lean::replace(
        "@[spec] def nz_from (v : u64) : RustM (core_models.num.nonzero.NonZero u64) :=
  pure (core_models.num.nonzero.nzMk v)"
    )
)]
pub fn nz_from(v: u64) -> NonZeroU64 {
    match NonZeroU64::new(v) {
        Some(n) => n,
        None => NonZeroU64::MIN,
    }
}

/// `Event` capacity accessor.
#[cfg_attr(
    hax,
    hax_lib::lean::replace(
        "@[spec] def get_capacity (e : ticketing_interface.state.event.Event) : RustM u64 :=
  pure e.capacity"
    )
)]
pub fn get_capacity(e: &Event) -> u64 {
    e.capacity.get()
}

/// `Event` unit-price accessor.
#[cfg_attr(
    hax,
    hax_lib::lean::replace(
        "@[spec] def get_price (e : ticketing_interface.state.event.Event) : RustM u64 :=
  pure e.price"
    )
)]
pub fn get_price(e: &Event) -> u64 {
    e.price_lamports.get()
}

/// `Event` sold-counter accessor.
#[cfg_attr(
    hax,
    hax_lib::lean::replace(
        "@[spec] def get_sold (e : ticketing_interface.state.event.Event) : RustM u64 :=
  pure e.sold"
    )
)]
pub fn get_sold(e: &Event) -> u64 {
    e.sold
}

/// "Sales are open" accessor.
#[cfg_attr(
    hax,
    hax_lib::lean::replace(
        "@[spec] def is_open (e : ticketing_interface.state.event.Event) : RustM Bool :=
  pure (!e.closed)"
    )
)]
pub fn is_open(e: &Event) -> bool {
    matches!(e.status, EventStatus::SalesOpen)
}

/// Organizer-equality accessor.
#[cfg_attr(hax, hax_lib::lean::replace(
    "@[spec] def organizer_eq (e : ticketing_interface.state.event.Event) (who : solana_address.Address) : RustM Bool :=
  pure (decide (e.organizer = who))"
))]
pub fn organizer_eq(e: &Event, who: &Address) -> bool {
    e.organizer == *who
}

/// Sold-counter update; callers keep `v <= capacity`, so the event invariant is
/// preserved. Only `sold` changes.
#[cfg_attr(hax, hax_lib::lean::replace(
    "@[spec] def set_sold (e : ticketing_interface.state.event.Event) (v : u64) : RustM ticketing_interface.state.event.Event :=
  pure { e with sold := v }"
))]
pub fn set_sold(e: &mut Event, v: u64) {
    e.sold = v;
}

/// Status update: `open -> closed`. Only `status` changes.
#[cfg_attr(hax, hax_lib::lean::replace(
    "@[spec] def set_status_closed (e : ticketing_interface.state.event.Event) : RustM ticketing_interface.state.event.Event :=
  pure { e with closed := true }"
))]
pub fn set_status_closed(e: &mut Event) {
    e.status = EventStatus::SalesClosed;
}

/// Builds a fresh sales-open event with a zeroed counter - the base case for
/// the event invariant. Capacity and price arrive as positive `u64` (the world
/// the accounting runs in) and are re-materialized as `NonZeroU64` here at the
/// wire boundary.
#[cfg_attr(hax, hax_lib::lean::replace(
    "@[spec] def event_new (organizer : solana_address.Address) (capacity : u64) (price_lamports : u64) (name : ticketing_interface.wire.bounded_string.BoundedString (64 : usize)) (bump : u8) (created_slot : u64) : RustM ticketing_interface.state.event.Event :=
  pure { organizer := organizer, capacity := capacity, price := price_lamports, sold := 0, closed := false, name := name, createdSlot := created_slot, updatedSlot := created_slot, bump := bump }"
))]
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
#[cfg_attr(hax, hax_lib::lean::replace(
    "@[spec] def ticket_new (event : solana_address.Address) (buyer : solana_address.Address) (paid_lamports : core_models.num.nonzero.NonZero u64) (quantity : core_models.num.nonzero.NonZero u64) (bump : u8) (created_slot : u64) : RustM ticketing_interface.state.ticket.Ticket :=
  pure { event := event, buyer := buyer, quantity := quantity, paid := paid_lamports, bump := bump, createdSlot := created_slot, updatedSlot := created_slot }"
))]
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
