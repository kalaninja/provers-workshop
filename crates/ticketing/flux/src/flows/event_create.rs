//! Verified decision core for the `event_create` instruction.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName};

use crate::domain::{
    event::event_new,
    extern_specs::{mul_fits, nz_get},
};

/// Creates an event, rejecting economics whose worst-case payment (`capacity *
/// price_lamports`) would overflow `u64` - the bound the event invariant then
/// carries for the account's whole life. On success the event is initial
/// (zeroed counter, pinned slots/organizer/bump, sales open).
#[flux_rs::sig(
    fn(
        organizer: Address[@o],
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8[@b],
        created_slot: u64[@slot],
    ) -> Result<Event{v:
        v.organizer == o
            && v.created_slot == slot
            && v.updated_slot == slot
            && v.sold == 0
            && v.bump == b
            && !v.closed
    }, ProgramError>
    requires is_set(o)
)]
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
    if !mul_fits(c, p) {
        return Err(ProgramError::ArithmeticOverflow);
    }
    Ok(event_new(organizer, c, p, name, bump, created_slot))
}
