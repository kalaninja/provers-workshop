//! Verified decision core for the `event_close` instruction.
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::Event;

use crate::domain::event::{close_sales, validate_organizer};

/// Closes an event: authorize the organizer, then move `SalesOpen ->
/// SalesClosed`. Immutable fields are fixed and the status step is legal.
#[flux_rs::sig(
    fn(event: &mut Event[@e], organizer: &Address) -> Result<(), ProgramError>
    ensures event: Event{new: same_except_status(e, new) && legal_status(e.closed, new.closed)}
)]
pub fn close_event(event: &mut Event, organizer: &Address) -> Result<(), ProgramError> {
    validate_organizer(event, organizer)?;
    close_sales(event)
}
