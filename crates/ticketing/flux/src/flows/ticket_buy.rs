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
/// A successful purchase increases `sold` by `min(requested, capacity -
/// old_sold)`. The ticket records that quantity and its exact payment, which
/// is also returned to the shell. Errors leave the modeled event unchanged.
#[flux_rs::sig(
    fn(
        event: &mut Event[@e],
        organizer: &Address[@o],
        event_addr: &Address[@ea],
        buyer: &Address[@b],
        requested: NonZeroU64[@r],
        bump: u8[@bp],
        created_slot: u64[@slot],
    ) -> Result<(
        NonZeroU64[int_min(r, e.capacity - e.sold) * e.price],
        Ticket[Ticket {
            event: ea,
            buyer: b,
            quantity: int_min(r, e.capacity - e.sold),
            paid_lamports: int_min(r, e.capacity - e.sold) * e.price,
            bump: bp,
            created_slot: slot,
            updated_slot: slot,
        }],
    ), ProgramError>[e.organizer == o && !e.closed && e.sold < e.capacity]
    requires is_set(ea) && is_set(b)
    ensures event: Event{new:
        same_except_sold(e, new) && new.sold >= e.sold
            && ((e.organizer != o || e.closed || e.sold == e.capacity) => new == e)
            && ((e.organizer == o && !e.closed && e.sold < e.capacity)
                => new.sold == e.sold + int_min(r, e.capacity - e.sold))
    }
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
    if let Err(err) = validate_organizer(event, organizer) {
        return Err(err);
    }
    let actual = match record_sale(event, requested) {
        Ok(actual) => actual,
        Err(err) => return Err(err),
    };
    let payment = nz_from(payment_for(event, actual));
    let ticket = ticket_new(*event_addr, *buyer, payment, nz_from(actual), bump, created_slot);
    Ok((payment, ticket))
}
