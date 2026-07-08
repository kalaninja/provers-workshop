//! Integration tests for the `event_close` instruction.

mod common;

use mollusk_svm::result::Check;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use ticketing_interface::EventStatus;

use crate::common::{
    DEFAULT_CAPACITY, DEFAULT_PRICE_LAMPORTS, PROGRAM_ID, WALLET_LAMPORTS, assert_program_error,
    derive_event_pda, event_close_instruction, event_close_payload, find_resulting_account,
    funded_system_account, mollusk, parse_event_state, seeded_event_account,
};

#[test]
fn closes_ticket_sales() {
    let mollusk = mollusk();
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
                42,
                EventStatus::SalesOpen,
            ),
        ),
    ];

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[Check::success(), Check::compute_units(588)],
    );

    let event_account =
        find_resulting_account(&result, &event_pda).expect("event account persists");
    let event = parse_event_state(&event_account.data);
    assert_eq!(event.status, EventStatus::SalesClosed);
    assert_eq!(event.sold, 42);
}

#[test]
fn rejects_non_organizer_authority() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([2; 32]);
    let impostor = Pubkey::new_from_array([102; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix = event_close_instruction(&impostor, &event_pda);
    let accounts = vec![
        (impostor, funded_system_account(WALLET_LAMPORTS)),
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

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidAccountData);
}

#[test]
fn rejects_double_close() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([3; 32]);
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
                EventStatus::SalesClosed,
            ),
        ),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidAccountData);
}

#[test]
fn rejects_non_signer_organizer() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([4; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(organizer, false),
            AccountMeta::new(event_pda, false),
        ],
        data: event_close_payload(),
    };
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

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::MissingRequiredSignature);
}
