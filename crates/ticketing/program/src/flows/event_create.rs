//! Verified decision core for the `event_create` instruction.
//!
//! The trusted shell creates the event PDA and persists the state; this core
//! validates the economics and constructs the initial event.

use core::num::NonZeroU64;

#[cfg(creusot)]
use creusot_std::prelude::*;
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName};

#[cfg(creusot)]
use crate::domain::AddressOps;
use crate::domain::{EventOps, NonZeroOps};

/// Build the initial event state from the create fields. Returns the event
/// for the shell to persist; the result is a sales-open event.
///
/// The overflow check on `capacity * price_lamports` establishes the economic
/// bound the event invariant carries for its whole life: every later payment
/// computation (`quantity * price_lamports` with `quantity <= capacity`) is
/// covered by it.
#[cfg_attr(creusot, requires(organizer.is_set()))]
#[cfg_attr(creusot, ensures(match result {
    Ok(event) => event.is_initial(organizer, capacity, price_lamports, name, bump, created_slot),
    Err(_) => capacity.int() * price_lamports.int() > u64::MAX@,
}))]
pub(crate) fn create_event(
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> Result<Event, ProgramError> {
    capacity.ax_checked_mul(price_lamports).ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(Event::new(organizer, capacity, price_lamports, name, bump, created_slot))
}
