//! Decision core for the `ticket_buy` instruction.
//!
//! The trusted `instructions::ticket_buy` shell parses accounts, creates the
//! ticket PDA, performs the lamport transfer CPI, and persists the accounts.
//! This core holds the pure purchase decision — validate the organizer, clamp
//! the sale, price it, and build the buyer's ticket — so every
//! constructor/mutator call runs at a checked call-site.

use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, Ticket};

#[cfg(kani)]
use crate::domain::AddressOps;
use crate::domain::{EventOps, TicketOps};

/// Purchase for a buyer: validate the organizer, record the clamped sale on the
/// event, price it, and build the buyer's ticket. Returns the payment (for the
/// shell's transfer CPI) and the freshly-built ticket (for the shell to
/// persist).
///
/// In the contract, `pre` (`old(*event)`) is the event at entry and `post`
/// (`*event`) is the event after the call — mirroring Creusot's `*event` /
/// `^event`. The clauses dereference `event` explicitly into owned locals so
/// Kani's post-state rewrite applies (it does not see through
/// `event.method()`).
#[cfg_attr(kani, kani::requires((*event).invariant()))]
#[cfg_attr(kani, kani::requires(event_addr.is_set() && buyer.is_set()))]
#[cfg_attr(kani, kani::modifies(event))]
#[cfg_attr(kani, kani::ensures(|_: &Result<(NonZeroU64, Ticket), ProgramError>| {
    let post = *event;
    let pre = old(*event);
    post.invariant() && post.is_legal_transition_from(pre)
}))]
#[cfg_attr(kani, kani::ensures(|result: &Result<(NonZeroU64, Ticket), ProgramError>| {
    let post = *event;
    let pre = old(*event);
    match result {
        Ok((payment, ticket)) => pre.is_status_sales_open()
            && pre.organizer == *organizer
            && ticket.is_initial(*event_addr, *buyer, *payment, ticket.quantity, bump, created_slot)
            && ticket.is_consistent_with(*event_addr, post)
            && post.sold == pre.sold + ticket.quantity.get()
            && ticket.quantity.get() == requested.get().min(pre.capacity.get() - pre.sold)
            && payment.get() == ticket.quantity.get() * pre.price_lamports.get(),
        Err(_) => true,
    }
}))]
#[cfg_attr(kani, kani::ensures(|result: &Result<(NonZeroU64, Ticket), ProgramError>| {
    let post = *event;
    let pre = old(*event);
    match result {
        Ok(_) => post == Event { sold: post.sold, ..pre },
        Err(_) => post == pre,
    }
}))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn purchase(
    event: &mut Event,
    organizer: &Address,
    event_addr: &Address,
    buyer: &Address,
    requested: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> Result<(NonZeroU64, Ticket), ProgramError> {
    event.validate_organizer(organizer)?;
    let actual = event.record_sale(requested)?;
    let payment = event.payment_for(actual)?;
    let ticket = Ticket::new(*event_addr, *buyer, payment, actual, bump, created_slot);
    Ok((payment, ticket))
}
