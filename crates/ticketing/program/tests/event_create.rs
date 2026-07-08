//! Integration tests for the `event_create` instruction.

mod common;

use mollusk_svm::result::Check;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use ticketing_interface::{
    Event, EventCreateArgs, EventStatus, program::state::StateAccount, wire::AccountHeaderSpec,
};

use crate::common::{
    DEFAULT_CAPACITY, DEFAULT_PRICE_LAMPORTS, PROGRAM_ID, WALLET_LAMPORTS, assert_program_error,
    derive_event_pda, empty_system_account, event_create_instruction, event_create_payload,
    event_name, find_resulting_account, funded_system_account, mollusk, nonzero, parse_event_state,
    rent_exempt_minimum, seeded_event_account, system_program_account,
};

fn default_args() -> EventCreateArgs {
    EventCreateArgs::new(nonzero(DEFAULT_CAPACITY), nonzero(DEFAULT_PRICE_LAMPORTS), event_name())
}

#[test]
fn creates_event_pda_with_program_owner_and_rent_exempt_balance() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([1; 32]);
    let (event_pda, expected_bump) = derive_event_pda(&organizer);

    let ix =
        event_create_instruction(&organizer, &event_pda, event_create_payload(&default_args()));
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::compute_units(4_948),
            Check::account(&event_pda)
                .owner(&PROGRAM_ID)
                .lamports(rent_exempt_minimum(Event::LEN))
                .build(),
        ],
    );

    let event_account =
        find_resulting_account(&result, &event_pda).expect("event account is created");
    let event = parse_event_state(&event_account.data);

    assert_eq!(Event::DISCRIMINATOR, *b"TKTEVNT");
    assert_eq!(Event::VERSION, 1);
    assert_eq!(event.organizer, organizer);
    assert_eq!(event.updated_slot, event.created_slot);
    assert_eq!(event.capacity.get(), DEFAULT_CAPACITY);
    assert_eq!(event.price_lamports.get(), DEFAULT_PRICE_LAMPORTS);
    assert_eq!(event.sold, 0);
    assert_eq!(event.name, event_name());
    assert_eq!(event.bump, expected_bump);
    assert_eq!(event.status, EventStatus::SalesOpen);
}

#[test]
fn rejects_non_signer_organizer() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([2; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(organizer, false),
            AccountMeta::new(event_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: event_create_payload(&default_args()),
    };
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::MissingRequiredSignature);
}

#[test]
fn rejects_second_event_for_same_organizer() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([3; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix =
        event_create_instruction(&organizer, &event_pda, event_create_payload(&default_args()));
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
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    // `SystemError::AccountAlreadyInUse` surfaced by the CreateAccountAllowPrefund
    // CPI.
    assert_program_error(&result.program_result, ProgramError::Custom(0));
}

#[test]
fn creates_event_pda_over_a_prefunded_slot() {
    // Anyone can send lamports to the derivable event PDA before it is created.
    // CreateAccountAllowPrefund tolerates a prefunded *empty* slot - it tops up
    // only the rent deficit - so creation still succeeds, closing the griefing
    // vector. A non-empty slot is still rejected (see
    // rejects_second_event_for_same_organizer).
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([6; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let ix =
        event_create_instruction(&organizer, &event_pda, event_create_payload(&default_args()));
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        // Prefunded with 1 lamport, empty data, system-owned: the griefing shape.
        (event_pda, funded_system_account(1)),
        system_program_account(),
    ];

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::compute_units(3_448),
            Check::account(&event_pda)
                .owner(&PROGRAM_ID)
                .lamports(rent_exempt_minimum(Event::LEN))
                .build(),
        ],
    );
}

#[test]
fn rejects_zero_capacity_payload() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([4; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let mut payload = event_create_payload(&default_args());
    payload[1..9].fill(0);

    let ix = event_create_instruction(&organizer, &event_pda, payload);
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidInstructionData);
}

#[test]
fn rejects_overflowing_economics() {
    let mollusk = mollusk();
    let organizer = Pubkey::new_from_array([5; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);

    let args = EventCreateArgs::new(nonzero(u64::MAX), nonzero(2), event_name());
    let ix = event_create_instruction(&organizer, &event_pda, event_create_payload(&args));
    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::ArithmeticOverflow);
}
