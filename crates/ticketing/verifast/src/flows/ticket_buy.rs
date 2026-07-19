//! Decision core for the `ticket_buy` instruction.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, Ticket};

use crate::domain::{EventOps, TicketOps};

/// Purchase for a buyer: validate the organizer, record the clamped sale, price
/// it, and build the buyer's ticket. Returns the payment (for the shell's
/// transfer CPI) and the ticket to persist.
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
