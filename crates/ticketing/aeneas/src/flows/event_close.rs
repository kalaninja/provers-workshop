//! Verified decision core for the `event_close` instruction.
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::Event;

use crate::domain::{event::close_sales, glue::organizer_eq};

/// Closes an event: authorize the organizer, then move `SalesOpen ->
/// SalesClosed`. Immutable fields are fixed and the status step is legal.
///
/// Proved in Lean: `Ok` exactly when the organizer matches and sales were open;
/// only `status` changes (open -> closed); on error the event is unchanged.
pub fn close_event(event: &mut Event, organizer: &Address) -> Result<(), ProgramError> {
    if !organizer_eq(event, organizer) {
        return Err(ProgramError::InvalidAccountData);
    }
    close_sales(event)
}
