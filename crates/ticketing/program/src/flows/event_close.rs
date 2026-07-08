//! Verified decision core for the `event_close` instruction.
//!
//! The trusted shell parses accounts and opens the event borrow; this core
//! authorizes the organizer and performs the status transition.

#[cfg(creusot)]
use creusot_std::prelude::*;
use pinocchio::{Address, ProgramResult};
use ticketing_interface::Event;

use crate::domain::EventOps;

/// Close ticket sales for the event after checking the supplied authority is
/// the event's organizer.
#[cfg_attr(creusot, requires((*event).invariant()))]
#[cfg_attr(creusot, ensures((^event).invariant()))]
#[cfg_attr(creusot, ensures((^event).is_legal_transition_from(*event)))]
#[cfg_attr(creusot, ensures(match result {
    Ok(_) => (*event).organizer == *organizer
        && (*event).is_status_sales_open()
        && (^event).is_status_sales_closed(),
    Err(_) => (*event).organizer != *organizer || !(*event).is_status_sales_open(),
}))]
#[cfg_attr(creusot, ensures(match result {
    Ok(_) => (^event) == Event { status: (^event).status, ..*event },
    Err(_) => (^event) == *event,
}))]
pub(crate) fn close_event(event: &mut Event, organizer: &Address) -> ProgramResult {
    event.validate_organizer(organizer)?;
    event.close_sales()
}
