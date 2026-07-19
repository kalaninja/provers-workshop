//! Decision core for the `event_close` instruction.
use pinocchio::{Address, ProgramResult};
use ticketing_interface::Event;

use crate::domain::EventOps;

/// Closes ticket sales after checking the supplied authority is the event's
/// organizer.
pub(crate) fn close_event(event: &mut Event, organizer: &Address) -> ProgramResult {
    event.validate_organizer(organizer)?;
    event.close_sales()
}
