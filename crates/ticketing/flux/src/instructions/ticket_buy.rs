use pinocchio::{
    AccountView, Address, ProgramResult,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_system::instructions::Transfer;
use ticketing_interface::{
    Event, Ticket, TicketBuyAccounts, TicketBuyArgs, program::state::PdaAccount,
};

use crate::flows::ticket_buy::purchase;

/// Sells tickets to the buyer: records the sale on the event, creates the
/// buyer's ticket PDA, and transfers the lamport payment to the organizer.
///
/// Exact account order: buyer, event, organizer, ticket PDA, system program.
///
/// Imperative shell: parses accounts, creates the ticket PDA, then delegates
/// the purchase decision to the `flows::ticket_buy` core before performing the
/// transfer CPI.
#[flux_rs::trusted]
pub(crate) fn process(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: &TicketBuyArgs,
) -> ProgramResult {
    let TicketBuyAccounts {
        buyer,
        event: event_account,
        organizer,
        ticket: ticket_account,
        system_program: _,
    } = TicketBuyAccounts::parse(accounts)?;

    let event_addr = *event_account.address();
    let slot = Clock::get()?.slot;

    let mut event = Event::mutate_account_at(program_id, event_account, slot)?;
    let bump = Ticket::create_pda_account(
        program_id,
        buyer,
        ticket_account,
        &(event_addr, *buyer.address()),
    )?;
    let (payment, ticket) = purchase(
        &mut event,
        organizer.address(),
        &event_addr,
        buyer.address(),
        args.quantity,
        bump,
        slot,
    )?;
    ticket.write_to_account(ticket_account)?;

    Transfer { from: buyer, to: organizer, lamports: payment.get() }.invoke()?;
    Ok(())
}
