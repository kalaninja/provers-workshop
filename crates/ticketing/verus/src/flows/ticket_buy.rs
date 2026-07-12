//! Verified decision core for the `ticket_buy` instruction.
//!
//! Holds the pure purchase decision - validate the organizer, clamp the sale,
//! price it, and build the buyer's ticket - so every constructor/mutator call
//! runs at a verified call site.

use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, Ticket};
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::math::min;
#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg(feature = "verus")]
use crate::domain::event::{event_invariant, is_legal_transition_from, is_status_sales_open};
#[cfg(feature = "verus")]
use crate::domain::extern_specs::is_set;
#[cfg(feature = "verus")]
use crate::domain::ticket::{ticket_is_consistent_with, ticket_is_initial};
use crate::domain::{
    event::{payment_for, record_sale, validate_organizer},
    ticket::ticket_new,
};

verus! {

/// Purchase for a buyer: validate the organizer, record the clamped sale, price
/// it, and build the buyer's ticket. Returns the payment (for the shell's
/// transfer CPI) and the ticket to persist.
#[allow(clippy::too_many_arguments)]
pub fn purchase(
    event: &mut Event,
    organizer: &Address,
    event_addr: &Address,
    buyer: &Address,
    requested: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> (result: Result<(NonZeroU64, Ticket), ProgramError>)
    requires
        event_invariant(*old(event)),
        is_set(*event_addr),
        is_set(*buyer),
    ensures
        event_invariant(*final(event)),
        is_legal_transition_from(*final(event), *old(event)),
        match result {
            Ok((payment, ticket)) => {
                &&& is_status_sales_open(*old(event))
                &&& old(event).organizer == *organizer
                &&& ticket_is_initial(
                    ticket,
                    *event_addr,
                    *buyer,
                    payment,
                    ticket.quantity,
                    bump,
                    created_slot,
                )
                &&& ticket_is_consistent_with(ticket, *event_addr, *final(event))
                &&& final(event).sold == old(event).sold + ticket.quantity@
                &&& ticket.quantity@ as int == min(
                    requested@ as int,
                    old(event).capacity@ - old(event).sold,
                )
                &&& payment@ == ticket.quantity@ * old(event).price_lamports@
                &&& *final(event) == (Event { sold: final(event).sold, ..*old(event) })
            },
            Err(_) => *final(event) == *old(event),
        },
{
    validate_organizer(event, organizer)?;
    let actual = record_sale(event, requested)?;
    let payment = payment_for(event, actual)?;
    let ticket = ticket_new(*event_addr, *buyer, payment, actual, bump, created_slot);
    Ok((payment, ticket))
}

} // verus!
