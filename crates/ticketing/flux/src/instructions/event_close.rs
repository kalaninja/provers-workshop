use pinocchio::{AccountView, Address, ProgramResult};
use ticketing_interface::{Event, EventCloseAccounts, program::state::PdaAccount};

use crate::flows::event_close::close_event;

/// Closes ticket sales for an event.
///
/// Exact account order: organizer, event.
///
/// Imperative shell: parses accounts and opens the event borrow, then delegates
/// the authorization and status transition to the `flows::event_close` core.
#[flux_rs::trusted]
pub(crate) fn process(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let EventCloseAccounts { organizer, event: event_account } =
        EventCloseAccounts::parse(accounts)?;

    let mut event = Event::mutate_account_at_current(program_id, event_account)?;
    close_event(&mut event, organizer.address())
}
