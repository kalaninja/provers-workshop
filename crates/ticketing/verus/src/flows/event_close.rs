//! Verified decision core for the `event_close` instruction.
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::Event;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::domain::event::{close_sales, validate_organizer};
#[cfg(feature = "verus")]
use crate::domain::event::{
    event_invariant, is_legal_transition_from, is_status_sales_closed, is_status_sales_open,
};

verus! {

/// Closes an event: authorize the organizer, then move `SalesOpen ->
/// SalesClosed`.
pub fn close_event(event: &mut Event, organizer: &Address) -> (result: Result<(), ProgramError>)
    requires
        event_invariant(*old(event)),
    ensures
        event_invariant(*final(event)),
        is_legal_transition_from(*final(event), *old(event)),
        match result {
            Ok(_) => {
                &&& old(event).organizer == *organizer
                &&& is_status_sales_open(*old(event))
                &&& is_status_sales_closed(*final(event))
                &&& *final(event) == (Event { status: final(event).status, ..*old(event) })
            },
            Err(_) => {
                &&& (old(event).organizer != *organizer || !is_status_sales_open(*old(event)))
                &&& *final(event) == *old(event)
            },
        },
{
    validate_organizer(event, organizer)?;
    close_sales(event)
}

} // verus!
