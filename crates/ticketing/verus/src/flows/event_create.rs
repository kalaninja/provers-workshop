//! Verified decision core for the `event_create` instruction.

use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName};
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg(feature = "verus")]
use crate::domain::event::event_is_initial;
use crate::domain::event::event_new;
#[cfg(feature = "verus")]
use crate::domain::extern_specs::is_set;

verus! {

/// Creates an event, rejecting economics whose worst-case payment (`capacity *
/// price_lamports`) would overflow `u64` - the bound the event invariant then
/// carries for the account's whole life.
pub fn create_event(
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> (result: Result<Event, ProgramError>)
    requires
        is_set(organizer),
    ensures
        match result {
            Ok(e) => event_is_initial(e, organizer, capacity, price_lamports, name, bump, created_slot),
            Err(_) => capacity@ * price_lamports@ > u64::MAX,
        },
{
    let c = capacity.get();
    let p = price_lamports.get();
    if c.checked_mul(p).is_none() {
        return Err(ProgramError::ArithmeticOverflow);
    }
    Ok(event_new(organizer, capacity, price_lamports, name, bump, created_slot))
}

} // verus!
