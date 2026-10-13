//! Verified decision core for the `event_close` instruction.
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::Event;

use crate::domain::event::{close_sales, validate_organizer};

/// Closes an event: authorize the organizer, then move `SalesOpen ->
/// SalesClosed`. Success requires the closed status; other fields stay fixed.
/// Errors leave the modeled event unchanged.
#[flux_rs::sig(
    fn(event: &mut Event[@e], organizer: &Address[@o]) -> Result<(), ProgramError>[e.organizer == o && !e.closed]
    ensures event: Event{new:
        same_except_status(e, new) && legal_status(e.closed, new.closed)
            && ((e.organizer == o && !e.closed) => new.closed)
            && ((e.organizer != o || e.closed) => new == e)
    }
)]
pub fn close_event(event: &mut Event, organizer: &Address) -> Result<(), ProgramError> {
    if let Err(err) = validate_organizer(event, organizer) {
        return Err(err);
    }
    close_sales(event)
}
