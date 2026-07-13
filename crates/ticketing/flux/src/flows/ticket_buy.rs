//! Verified decision core for the `ticket_buy` instruction.
//!
//! Holds the pure purchase decision - validate the organizer, clamp the sale,
//! price it, and build the buyer's ticket - so every constructor/mutator call
//! runs at a verified call site. The event invariant is preserved (it rides on
//! the `Event` type), immutable fields are fixed, and `sold` only grows.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, Ticket};

use crate::domain::{
    event::{payment_for, record_sale, validate_organizer},
    extern_specs::nz_from,
    ticket::ticket_new,
};

/// Purchase for a buyer: validate the organizer, record the clamped sale, price
/// it, and build the buyer's ticket. Returns the payment (for the shell's
/// transfer CPI) and the ticket to persist.
#[flux_rs::sig(
    fn(
        event: &mut Event[@e],
        organizer: &Address,
        event_addr: &Address[@ea],
        buyer: &Address[@b],
        requested: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> Result<(NonZeroU64, Ticket), ProgramError>
    requires is_set(ea) && is_set(b)
    ensures event: Event{new: same_except_sold(e, new) && new.sold >= e.sold}
)]
pub fn purchase(
    event: &mut Event,
    organizer: &Address,
    event_addr: &Address,
    buyer: &Address,
    requested: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> Result<(NonZeroU64, Ticket), ProgramError> {
    validate_organizer(event, organizer)?;
    let actual = record_sale(event, requested)?;
    let payment = nz_from(payment_for(event, actual));
    let ticket = ticket_new(*event_addr, *buyer, payment, nz_from(actual), bump, created_slot);
    Ok((payment, ticket))
}
