//! Verified decision core for the `ticket_buy` instruction.
//!
//! The trusted `instructions::ticket_buy` shell parses accounts, creates the
//! ticket PDA, performs the lamport transfer CPI, and persists the accounts.
//! This core holds the pure purchase decision — validate the organizer, clamp
//! the sale, price it, and build the buyer's ticket — so every
//! constructor/mutator call runs at a verified call-site.

use core::num::NonZeroU64;

#[cfg(creusot)]
use creusot_std::prelude::*;
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, Ticket};

#[cfg(creusot)]
use crate::domain::{AddressOps, NonZeroOps, int_min};
use crate::domain::{EventOps, TicketOps};

/// Purchase for a buyer: validate the organizer, record the clamped sale on
/// the event, price it, and build the buyer's ticket. Returns the payment
/// (for the shell's transfer CPI) and the freshly-built ticket (for the shell
/// to persist).
#[cfg_attr(creusot, requires((*event).invariant()))]
#[cfg_attr(creusot, requires((*event_addr).is_set() && (*buyer).is_set()))]
#[cfg_attr(creusot, ensures((^event).invariant()))]
#[cfg_attr(creusot, ensures((^event).is_legal_transition_from(*event)))]
#[cfg_attr(creusot, ensures(match result {
    Ok((payment, ticket)) => (*event).is_status_sales_open()
        && (*event).organizer == *organizer
        && ticket.is_initial(*event_addr, *buyer, payment, ticket.quantity, bump, created_slot)
        && ticket.is_consistent_with(*event_addr, ^event)
        && (^event).sold@ == (*event).sold@ + ticket.quantity.int()
        && ticket.quantity.int() == int_min(
            requested.int(),
            (*event).capacity.int() - (*event).sold@,
        )
        && payment.int() == ticket.quantity.int() * (*event).price_lamports.int(),
    Err(_) => true,
}))]
#[cfg_attr(creusot, ensures(match result {
    Ok(_) => (^event) == Event { sold: (^event).sold, ..*event },
    Err(_) => (^event) == *event,
}))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn purchase(
    event: &mut Event,
    organizer: &Address,
    event_addr: &Address,
    buyer: &Address,
    requested: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> Result<(NonZeroU64, Ticket), ProgramError> {
    event.validate_organizer(organizer)?;
    let actual = event.record_sale(requested)?;
    let payment = event.payment_for(actual)?;
    let ticket = Ticket::new(*event_addr, *buyer, payment, actual, bump, created_slot);
    Ok((payment, ticket))
}
