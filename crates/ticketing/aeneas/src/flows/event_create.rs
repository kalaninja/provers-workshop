//! Verified decision core for the `event_create` instruction.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName};

use crate::domain::glue::{event_new, nz_get};

/// Creates an event, rejecting economics whose worst-case payment (`capacity *
/// price_lamports`) would overflow `u64` - the bound the event invariant then
/// carries for the account's whole life. On success the event is initial
/// (zeroed counter, pinned slots/organizer/bump, sales open) and satisfies the
/// invariant.
///
/// Proved in Lean (`verif/ticketing_aeneas`): `Ok` yields an initial event
/// satisfying the invariant, and the error occurs exactly when `capacity *
/// price_lamports > u64::MAX`. The overflow check runs on `u64` via
/// `checked_mul` (a modeled operation), so the bound is proved, not trusted.
pub fn create_event(
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> Result<Event, ProgramError> {
    let c = nz_get(capacity);
    let p = nz_get(price_lamports);
    match c.checked_mul(p) {
        None => Err(ProgramError::ArithmeticOverflow),
        Some(_) => Ok(event_new(organizer, c, p, name, bump, created_slot)),
    }
}
