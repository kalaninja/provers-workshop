//! Integration tests focused on the slot-pinned mutation guard exposed by
//! `PdaAccount::mutate_account_at` / `mutate_account_at_current`. They
//! exercise the slot path through the `event_close` and `ticket_buy`
//! handlers.

mod common;

use mollusk_svm::result::Check;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use ticketing_interface::{EventStatus, TicketBuyArgs};

use crate::common::{
    DEFAULT_CAPACITY, DEFAULT_PRICE_LAMPORTS, WALLET_LAMPORTS, assert_program_error,
    derive_event_pda, derive_ticket_pda, empty_system_account, event_close_instruction,
    find_resulting_account, funded_system_account, mollusk, nonzero, parse_event_state,
    parse_ticket_state, seeded_event_account, seeded_event_account_updated_at,
    system_program_account, ticket_buy_instruction, ticket_buy_payload,
};

#[test]
fn advances_slot_to_clock_value() {
    let mut mollusk = mollusk();
    mollusk.warp_to_slot(20);

    let organizer = Pubkey::new_from_array([1; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix = event_close_instruction(&organizer, &event_pda);
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (
            event_pda,
            seeded_event_account(
                &organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
            ),
        ),
    ];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);

    let event_account =
        find_resulting_account(&result, &event_pda).expect("event account persists");
    let event = parse_event_state(&event_account.data);
    assert_eq!(event.updated_slot, 20);
    assert_eq!(event.created_slot, 0, "created_slot is never touched by mutations");
}

#[test]
fn accepts_equal_slot() {
    let mut mollusk = mollusk();
    mollusk.warp_to_slot(20);

    let organizer = Pubkey::new_from_array([2; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix = event_close_instruction(&organizer, &event_pda);
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (
            event_pda,
            seeded_event_account_updated_at(
                &organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
                20,
            ),
        ),
    ];

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);

    let event_account =
        find_resulting_account(&result, &event_pda).expect("event account persists");
    assert_eq!(parse_event_state(&event_account.data).updated_slot, 20);
}

#[test]
fn rejects_slot_regression() {
    let mut mollusk = mollusk();
    mollusk.warp_to_slot(50);

    let organizer = Pubkey::new_from_array([3; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix = event_close_instruction(&organizer, &event_pda);
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (
            event_pda,
            seeded_event_account_updated_at(
                &organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
                100,
            ),
        ),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidArgument);
}

#[test]
fn chained_mutations_track_each_advance() {
    let mut mollusk = mollusk();

    let organizer = Pubkey::new_from_array([4; 32]);
    let buyer = Pubkey::new_from_array([104; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);
    let (ticket_pda, _) = derive_ticket_pda(&event_pda, &buyer);

    // Step 1: buy tickets at slot 10 — the event mutation pins the clock slot,
    // and the freshly created ticket records it as created/updated.
    mollusk.warp_to_slot(10);

    let buy_ix = ticket_buy_instruction(
        &buyer,
        &event_pda,
        &organizer,
        &ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(3))),
    );
    let buy_accounts = vec![
        (buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            event_pda,
            seeded_event_account(
                &organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
            ),
        ),
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let buy_result =
        mollusk.process_and_validate_instruction(&buy_ix, &buy_accounts, &[Check::success()]);

    let event_after_buy =
        find_resulting_account(&buy_result, &event_pda).expect("event account persists");
    assert_eq!(parse_event_state(&event_after_buy.data).updated_slot, 10);
    let ticket_account =
        find_resulting_account(&buy_result, &ticket_pda).expect("ticket account is created");
    let ticket = parse_ticket_state(&ticket_account.data);
    assert_eq!(ticket.created_slot, 10);
    assert_eq!(ticket.updated_slot, 10);

    // Step 2: close sales at slot 25, starting from the post-buy event state.
    mollusk.warp_to_slot(25);

    let close_ix = event_close_instruction(&organizer, &event_pda);
    let close_accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, event_after_buy.clone()),
    ];

    let close_result =
        mollusk.process_and_validate_instruction(&close_ix, &close_accounts, &[Check::success()]);

    let event_after_close =
        find_resulting_account(&close_result, &event_pda).expect("event account persists");
    let event = parse_event_state(&event_after_close.data);
    assert_eq!(event.updated_slot, 25);
    assert_eq!(event.sold, 3);
    assert_eq!(event.status, EventStatus::SalesClosed);
}
