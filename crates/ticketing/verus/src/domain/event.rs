//! Event predicates and the verified lifecycle/accounting mutators. Mirrors the
//! Creusot lane's `EventOps` 1:1 in behavior; the ghost layer (spec fns,
//! contracts, proofs) erases from ordinary/SBF builds.

use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName, EventStatus};
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::math::min;
#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg(feature = "verus")]
use crate::domain::extern_specs::is_set;
use crate::domain::extern_specs::{account_header, nz_checked_mul, nz_min, zero_reserved};

verus! {

// --- predicates -----------------------------------------------------------------------------

/// Whether ticket sales are open.
pub open spec fn is_status_sales_open(e: Event) -> bool {
    e.status == EventStatus::SalesOpen
}

/// Whether ticket sales are closed.
pub open spec fn is_status_sales_closed(e: Event) -> bool {
    e.status == EventStatus::SalesClosed
}

/// Event invariant: the organizer is set, sold never exceeds capacity, and the worst-case payment
/// (`capacity * price_lamports`) fits in `u64`.
pub open spec fn event_invariant(e: Event) -> bool {
    &&& is_set(e.organizer)
    &&& e.sold <= e.capacity@
    &&& e.capacity@ * e.price_lamports@ <= u64::MAX
}

/// Legal event status steps: `SalesOpen -> SalesClosed`, plus each status to itself.
pub open spec fn is_legal_event_status_transition(from: EventStatus, to: EventStatus) -> bool {
    ||| from == to
    ||| (from == EventStatus::SalesOpen && to == EventStatus::SalesClosed)
}

/// The complete initial event state written by [`event_new`].
pub open spec fn event_is_initial(
    e: Event,
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> bool {
    &&& e.sold == 0
    &&& e.created_slot == created_slot
    &&& e.updated_slot == created_slot
    &&& e.organizer == organizer
    &&& e.capacity == capacity
    &&& e.price_lamports == price_lamports
    &&& e.name == name
    &&& e.bump == bump
    &&& event_invariant(e)
    &&& is_status_sales_open(e)
}

/// Whether `e` is a legal successor of `prev`: immutable fields fixed, sold monotone, legal status.
pub open spec fn is_legal_transition_from(e: Event, prev: Event) -> bool {
    &&& e.header == prev.header
    &&& e.created_slot == prev.created_slot
    &&& e.organizer == prev.organizer
    &&& e.capacity == prev.capacity
    &&& e.price_lamports == prev.price_lamports
    &&& e.name == prev.name
    &&& e.bump == prev.bump
    &&& e._reserved == prev._reserved
    &&& e.sold >= prev.sold
    &&& is_legal_event_status_transition(prev.status, e.status)
}

// --- verified mutators ----------------------------------------------------------------------

/// Builds a fresh sales-open event with a zeroed counter - the base case for
/// [`event_invariant`].
pub fn event_new(
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> (r: Event)
    requires
        is_set(organizer),
        capacity@ * price_lamports@ <= u64::MAX,
    ensures
        event_is_initial(r, organizer, capacity, price_lamports, name, bump, created_slot),
{
    Event {
        header: account_header::<Event>(),
        created_slot,
        updated_slot: created_slot,
        organizer,
        capacity,
        price_lamports,
        sold: 0,
        name,
        bump,
        status: EventStatus::SalesOpen,
        _reserved: zero_reserved::<4>(),
    }
}

/// Returns `Ok` when the supplied authority is this event's organizer.
pub fn validate_organizer(e: &Event, organizer: &Address) -> (r: Result<(), ProgramError>)
    ensures
        match r {
            Ok(_) => e.organizer == *organizer,
            Err(_) => e.organizer != *organizer,
        },
{
    if e.organizer == *organizer { Ok(()) } else { Err(ProgramError::InvalidAccountData) }
}

/// Returns `Ok` when this event's ticket sales are open.
#[allow(clippy::match_wildcard_for_single_variants)]
pub fn require_sales_open(e: &Event) -> (r: Result<(), ProgramError>)
    ensures
        match r {
            Ok(_) => is_status_sales_open(*e),
            Err(_) => !is_status_sales_open(*e),
        },
{
    match e.status {
        EventStatus::SalesOpen => Ok(()),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

/// Records a ticket sale, clamping `requested` to remaining capacity and
/// returning the quantity actually sold.
pub fn record_sale(
    event: &mut Event,
    requested: NonZeroU64,
) -> (result: Result<NonZeroU64, ProgramError>)
    requires
        event_invariant(*old(event)),
    ensures
        event_invariant(*final(event)),
        match result {
            Ok(actual) => {
                &&& is_status_sales_open(*old(event))
                &&& actual@ as int == min(
                    requested@ as int,
                    old(event).capacity@ - old(event).sold,
                )
                &&& actual@ * old(event).price_lamports@ <= u64::MAX
                &&& final(event).sold == old(event).sold + actual@
                &&& *final(event) == (Event { sold: final(event).sold, ..*old(event) })
            },
            Err(_) => {
                &&& (!is_status_sales_open(*old(event))
                    || old(event).sold == old(event).capacity@)
                &&& *final(event) == *old(event)
            },
        },
{
    require_sales_open(event)?;

    let cap = event.capacity.get();
    let Some(remaining_u64) = cap.checked_sub(event.sold) else {
        return Err(ProgramError::InvalidAccountData);
    };
    let Some(remaining) = NonZeroU64::new(remaining_u64) else {
        return Err(ProgramError::InvalidAccountData);
    };
    let actual = nz_min(remaining, requested);

    proof {
        assert(actual@ * event.price_lamports@ <= u64::MAX) by (nonlinear_arith)
            requires
                actual@ <= event.capacity@,
                event.capacity@ * event.price_lamports@ <= u64::MAX;
    }

    let Some(new_sold) = event.sold.checked_add(actual.get()) else {
        return Err(ProgramError::ArithmeticOverflow);
    };
    event.sold = new_sold;
    Ok(actual)
}

/// Computes the lamport payment due for `quantity` tickets at this event's unit
/// price.
pub fn payment_for(e: &Event, quantity: NonZeroU64) -> (result: Result<NonZeroU64, ProgramError>)
    ensures
        match result {
            Ok(payment) => payment@ == quantity@ * e.price_lamports@,
            Err(_) => quantity@ * e.price_lamports@ > u64::MAX,
        },
{
    match nz_checked_mul(quantity, e.price_lamports) {
        Some(payment) => Ok(payment),
        None => Err(ProgramError::ArithmeticOverflow),
    }
}

/// Closes ticket sales.
pub fn close_sales(event: &mut Event) -> (result: Result<(), ProgramError>)
    requires
        event_invariant(*old(event)),
    ensures
        event_invariant(*final(event)),
        match result {
            Ok(_) => {
                &&& is_status_sales_open(*old(event))
                &&& is_status_sales_closed(*final(event))
                &&& *final(event) == (Event { status: final(event).status, ..*old(event) })
            },
            Err(_) => {
                &&& !is_status_sales_open(*old(event))
                &&& *final(event) == *old(event)
            },
        },
{
    require_sales_open(event)?;
    event.status = EventStatus::SalesClosed;
    Ok(())
}

} // verus!
