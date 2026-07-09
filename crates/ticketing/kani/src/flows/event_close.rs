//! Decision core for the `event_close` instruction.
//!
//! The trusted shell parses accounts and opens the event borrow; this core
//! authorizes the organizer and performs the status transition.

use pinocchio::{Address, ProgramResult};
use ticketing_interface::Event;

use crate::domain::EventOps;

/// Close ticket sales for the event after checking the supplied authority is
/// the event's organizer.
///
/// In the contract, `pre` (`old(*event)`) is the event at entry and `post`
/// (`*event`) is the event after the call — mirroring Creusot's `*event` /
/// `^event`. The clauses dereference `event` explicitly into owned locals so
/// Kani's post-state rewrite applies (it does not see through
/// `event.method()`).
#[cfg_attr(kani, kani::requires((*event).invariant()))]
#[cfg_attr(kani, kani::modifies(event))]
#[cfg_attr(kani, kani::ensures(|_: &ProgramResult| {
    let post = *event;
    let pre = old(*event);
    post.invariant() && post.is_legal_transition_from(pre)
}))]
#[cfg_attr(kani, kani::ensures(|result: &ProgramResult| {
    let post = *event;
    let pre = old(*event);
    match result {
        Ok(()) => pre.organizer == *organizer
            && pre.is_status_sales_open()
            && post.is_status_sales_closed()
            && post == Event { status: post.status, ..pre },
        Err(_) => {
            (pre.organizer != *organizer || !pre.is_status_sales_open()) && post == pre
        },
    }
}))]
pub(crate) fn close_event(event: &mut Event, organizer: &Address) -> ProgramResult {
    event.validate_organizer(organizer)?;
    event.close_sales()
}
