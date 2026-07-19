//! Decision core for the `event_create` instruction.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName};

use crate::domain::EventOps;

/// Builds the initial event state, rejecting economics whose worst-case payment
/// (`capacity * price_lamports`) would overflow `u64` - the bound the event
/// invariant then carries for the account's whole life.
pub(crate) fn create_event(
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> Result<Event, ProgramError> {
    capacity.checked_mul(price_lamports).ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(Event::new(organizer, capacity, price_lamports, name, bump, created_slot))
}
