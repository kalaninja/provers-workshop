//! Ticket predicates and the verified ticket constructor. Mirrors the Creusot
//! lane's `TicketOps`; the ghost layer erases from ordinary/SBF builds.
use core::num::NonZeroU64;

use pinocchio::Address;
#[cfg(feature = "verus")]
use ticketing_interface::Event;
use ticketing_interface::Ticket;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg(feature = "verus")]
use crate::domain::extern_specs::is_set;
use crate::domain::extern_specs::{account_header, zero_reserved};

verus! {

// --- predicates -----------------------------------------------------------------------------
/// Ticket invariant: both the event and buyer addresses are set.
pub open spec fn ticket_invariant(t: Ticket) -> bool {
    is_set(t.event) && is_set(t.buyer)
}

/// The complete initial ticket state written by [`ticket_new`].
pub open spec fn ticket_is_initial(
    t: Ticket,
    event: Address,
    buyer: Address,
    paid_lamports: NonZeroU64,
    quantity: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> bool {
    &&& t.event == event
    &&& t.buyer == buyer
    &&& t.paid_lamports == paid_lamports
    &&& t.quantity == quantity
    &&& t.created_slot == created_slot
    &&& t.updated_slot == created_slot
    &&& t.bump == bump
    &&& ticket_invariant(t)
}

/// Whether the ticket is consistent with the event it was sold against: it references that event
/// and its quantity does not exceed the event's recorded sales.
pub open spec fn ticket_is_consistent_with(t: Ticket, event_addr: Address, e: Event) -> bool {
    t.event == event_addr && t.quantity@ <= e.sold
}

// --- verified constructor -------------------------------------------------------------------
/// Builds a buyer's ticket for a priced sale.
pub fn ticket_new(
    event: Address,
    buyer: Address,
    paid_lamports: NonZeroU64,
    quantity: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> (r: Ticket)
    requires
        is_set(event),
        is_set(buyer),
    ensures
        ticket_is_initial(r, event, buyer, paid_lamports, quantity, bump, created_slot),
{
    Ticket {
        header: account_header::<Ticket>(),
        created_slot,
        updated_slot: created_slot,
        event,
        buyer,
        quantity,
        paid_lamports,
        bump,
        _reserved: zero_reserved::<7>(),
    }
}

} // verus!
