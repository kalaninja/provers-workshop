use pinocchio::{
    AccountView, Address, ProgramResult,
    sysvars::{Sysvar, clock::Clock},
};
use ticketing_interface::{
    Event, EventCreateAccounts, EventCreateArgs, program::state::PdaAccount,
};

use crate::flows::event_create::create_event;

/// Creates the canonical event PDA and writes the initial event state.
///
/// Exact account order: organizer, event PDA, system program.
///
/// Imperative shell: parses accounts, creates the PDA, reads the clock, then
/// delegates the construction decision to the verified `flows::event_create`
/// core before persisting the result.
pub(crate) fn process(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: &EventCreateArgs,
) -> ProgramResult {
    let EventCreateAccounts { organizer, event, system_program: _ } =
        EventCreateAccounts::parse(accounts)?;

    let bump = Event::create_pda_account(program_id, organizer, event, organizer.address())?;
    let created_slot = Clock::get()?.slot;
    create_event(
        *organizer.address(),
        args.capacity,
        args.price_lamports,
        args.name,
        bump,
        created_slot,
    )?
    .write_to_account(event)
}
