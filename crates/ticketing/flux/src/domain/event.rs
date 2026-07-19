//! Event predicates (carried as the `Event` type invariant) and the verified
//! lifecycle/accounting mutators. These implement the same decisions as the
//! other lanes' `EventOps`; refinement signatures erase from ordinary/SBF
//! builds.
//!
//! Accounting runs on `u64` (Flux models no `NonZeroU64`): the cores read the
//! event's fields as refined `u64` through the trusted accessors, so the clamp,
//! overflow, and no-oversell reasoning is machine-checked; `NonZeroU64` values
//! are re-materialized at the wire boundary via [`nz_from`].
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{
    Event, EventName, EventStatus,
    wire::{AccountHeader, Reserved},
};

use crate::domain::extern_specs::{
    get_capacity, get_price, get_sold, is_open, mul_le_mono, mul_pos, nz_from, nz_get,
    organizer_eq, set_sold, set_status_closed,
};

/// Builds a fresh sales-open event with a zeroed counter - the base case for
/// the `Event` invariant. Trusted: it constructs the opaque wire struct.
/// Capacity and price arrive as refined `u64` (the world the accounting runs
/// in) so the signature can demand the bounded-economics precondition -
/// `create_event` must prove `capacity * price <= u64::MAX` before an event can
/// be constructed - and are re-materialized as `NonZeroU64` at this wire
/// boundary.
#[flux_rs::trusted]
#[flux_rs::sig(
    fn(
        organizer: Address[@o],
        capacity: u64[@c],
        price_lamports: u64[@p],
        name: EventName,
        bump: u8[@b],
        created_slot: u64[@slot],
    ) -> Event[Event {
        organizer: o,
        created_slot: slot,
        updated_slot: slot,
        capacity: c,
        price: p,
        sold: 0,
        bump: b,
        closed: false,
    }]
    requires is_set(o) && 0 < c && 0 < p && c * p <= u64::MAX
)]
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

/// Returns `Ok` when the supplied authority is this event's organizer.
pub fn validate_organizer(e: &Event, organizer: &Address) -> Result<(), ProgramError> {
    if organizer_eq(e, organizer) { Ok(()) } else { Err(ProgramError::InvalidAccountData) }
}

/// Returns `Ok` when this event's ticket sales are open.
pub fn require_sales_open(e: &Event) -> Result<(), ProgramError> {
    if is_open(e) { Ok(()) } else { Err(ProgramError::InvalidAccountData) }
}

/// Records a ticket sale, clamping `requested` to remaining capacity and
/// returning the quantity actually sold. Only `sold` changes and it never
/// exceeds capacity (proven: the returned quantity fits in the remaining
/// headroom); `sold` is monotone and the other fields are pinned.
#[flux_rs::sig(
    fn(event: &mut Event[@e], requested: NonZeroU64)
        -> Result<u64{a: 0 < a && a <= e.capacity - e.sold}, ProgramError>
    ensures event: Event{new: same_except_sold(e, new) && new.sold >= e.sold}
)]
pub fn record_sale(event: &mut Event, requested: NonZeroU64) -> Result<u64, ProgramError> {
    require_sales_open(event)?;

    let cap = get_capacity(event);
    let sold = get_sold(event);

    // The invariant gives `sold <= capacity`, so this cannot underflow; and
    // `sold + actual <= sold + remaining == capacity <= u64::MAX`, so recording
    // the sale cannot overflow. Flux discharges both from the bounds below.
    let remaining = cap - sold;
    if remaining == 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    let requested = nz_get(requested);
    let actual = if remaining <= requested { remaining } else { requested };

    set_sold(event, sold + actual);
    Ok(actual)
}

/// Computes the lamport payment due for `quantity` tickets at this event's unit
/// price. Total, not fallible: the clamp guarantees `quantity <= capacity`, so
/// the product neither overflows (`capacity * price <= u64::MAX` is the event
/// invariant) nor is zero (both factors are positive). The signature states the
/// exact product `quantity * price`; its fit in `u64` and its positivity ride
/// the trusted `mul_le_mono`/`mul_pos` lemmas (flux-core's std specs are not
/// visible downstream), so no error case survives.
#[flux_rs::sig(
    fn(e: &Event[@ev], quantity: u64[@q]) -> u64{p: p == q * ev.price && 0 < p}
    requires 0 < q && q <= ev.capacity
)]
pub fn payment_for(e: &Event, quantity: u64) -> u64 {
    let price = get_price(e);
    mul_le_mono(quantity, get_capacity(e), price);
    mul_pos(quantity, price);
    quantity * price
}

/// Closes ticket sales. Only `status` changes; `open -> closed`.
#[flux_rs::sig(
    fn(event: &mut Event[@e]) -> Result<(), ProgramError>
    ensures event: Event{new: same_except_status(e, new) && legal_status(e.closed, new.closed)}
)]
pub fn close_sales(event: &mut Event) -> Result<(), ProgramError> {
    require_sales_open(event)?;
    set_status_closed(event);
    Ok(())
}
