//! SBF-executing integration tests for the generated client.
//!
//! Unlike `wire_parity.rs` (which only diffs encodings against the interface),
//! these run the real `ticketing_creusot.so` under Mollusk, driving the whole
//! event lifecycle **entirely through the generated client's instruction
//! builders + PDA finders**, then decoding every resulting account with the
//! client's decoders. This exercises all three instruction builders and both
//! account decoders end to end.
//!
//! The `svm-tests` feature gates this test target via `required-features` (the
//! same pattern as the program's `bpf-entrypoint`), so the default
//! `cargo test` / `just rust test` lane skips it: it needs the SBF artefact on
//! `SBF_OUT_DIR`. Run it via `just solana test-client-rust`.

use mollusk_svm::{Mollusk, result::Check};
use solana_address::Address;
use solana_instruction::Instruction;
use ticketing_client::{
    CheckedAccount, TICKETING_ID,
    accounts::{Event, Ticket},
    instructions::{
        EventClose, EventCreate, EventCreateInstructionArgs, TicketBuy, TicketBuyInstructionArgs,
    },
    types::{EventName, EventStatus},
};
use ticketing_interface::test_utils::{
    WALLET_LAMPORTS, empty_system_account, find_resulting_account, funded_system_account,
    mollusk_for, rent_exempt_minimum, system_program_account,
};

/// SBF artefact name Mollusk loads (Cargo normalises the package name to
/// this).
const PROGRAM_NAME: &str = "ticketing_creusot";

const CAPACITY: u64 = 500;
const PRICE_LAMPORTS: u64 = 2_000_000;
const QUANTITY: u64 = 3;

fn event_name(bytes: &[u8]) -> EventName {
    let mut storage = [0u8; 64];
    storage[..bytes.len()].copy_from_slice(bytes);
    EventName { len: u16::try_from(bytes.len()).expect("name fits"), bytes: storage }
}

#[test]
fn full_lifecycle_via_client() {
    let mollusk: Mollusk = mollusk_for(&TICKETING_ID, PROGRAM_NAME);

    let organizer = Address::from([30; 32]);
    let buyer = Address::from([32; 32]);
    let (event_pda, _) = Event::find_pda(&organizer);
    let (ticket_pda, _) = Ticket::find_pda(&event_pda, &buyer);
    let (system_program_id, system_program_entry) = system_program_account();

    let instructions = [
        EventCreate { organizer, event: event_pda, system_program: system_program_id }.instruction(
            EventCreateInstructionArgs {
                capacity: CAPACITY,
                price_lamports: PRICE_LAMPORTS,
                name: event_name(b"rust-summit-2026"),
                reserved: [0; 6],
            },
        ),
        TicketBuy {
            buyer,
            event: event_pda,
            organizer,
            ticket: ticket_pda,
            system_program: system_program_id,
        }
        .instruction(TicketBuyInstructionArgs { quantity: QUANTITY }),
        EventClose { organizer, event: event_pda }.instruction(),
    ];

    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (buyer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, empty_system_account()),
        (ticket_pda, empty_system_account()),
        (system_program_id, system_program_entry),
    ];

    let checks = [Check::success()];
    let chain: Vec<(&Instruction, &[Check])> =
        instructions.iter().map(|ix| (ix, &checks[..])).collect();

    let result = mollusk.process_and_validate_instruction_chain(&chain, &accounts);

    // Event decoder: sales closed after the full chain, with the purchase
    // recorded.
    let event_acct = find_resulting_account(&result, &event_pda).expect("event created");
    let event = Event::from_bytes(&event_acct.data).expect("event decodes");
    assert_eq!(event.organizer, organizer);
    assert_eq!(event.capacity, CAPACITY);
    assert_eq!(event.price_lamports, PRICE_LAMPORTS);
    assert_eq!(event.sold, QUANTITY);
    assert_eq!(event.status, EventStatus::SalesClosed);

    // Ticket decoder, through the checked path.
    let ticket_acct = find_resulting_account(&result, &ticket_pda).expect("ticket created");
    let ticket = Ticket::from_bytes_checked(&ticket_acct.data).expect("ticket decodes");
    assert_eq!(ticket.event, event_pda);
    assert_eq!(ticket.buyer, buyer);
    assert_eq!(ticket.quantity, QUANTITY);
    assert_eq!(ticket.paid_lamports, QUANTITY * PRICE_LAMPORTS);

    // The lamport payment landed with the organizer (who funded the event
    // rent out of the same wallet during `event_create`).
    let organizer_acct = find_resulting_account(&result, &organizer).expect("organizer present");
    assert_eq!(
        organizer_acct.lamports,
        WALLET_LAMPORTS - rent_exempt_minimum(event_acct.data.len()) + QUANTITY * PRICE_LAMPORTS
    );
}

#[test]
fn ticket_buy_via_client_is_rejected_after_close() {
    let mollusk: Mollusk = mollusk_for(&TICKETING_ID, PROGRAM_NAME);

    let organizer = Address::from([40; 32]);
    let buyer = Address::from([42; 32]);
    let (event_pda, _) = Event::find_pda(&organizer);
    let (ticket_pda, _) = Ticket::find_pda(&event_pda, &buyer);
    let (system_program_id, system_program_entry) = system_program_account();

    let instructions = [
        EventCreate { organizer, event: event_pda, system_program: system_program_id }.instruction(
            EventCreateInstructionArgs {
                capacity: CAPACITY,
                price_lamports: PRICE_LAMPORTS,
                name: event_name(b"rust-summit-2026"),
                reserved: [0; 6],
            },
        ),
        EventClose { organizer, event: event_pda }.instruction(),
        TicketBuy {
            buyer,
            event: event_pda,
            organizer,
            ticket: ticket_pda,
            system_program: system_program_id,
        }
        .instruction(TicketBuyInstructionArgs { quantity: QUANTITY }),
    ];

    let accounts = vec![
        (organizer, funded_system_account(WALLET_LAMPORTS)),
        (buyer, funded_system_account(WALLET_LAMPORTS)),
        (event_pda, empty_system_account()),
        (ticket_pda, empty_system_account()),
        (system_program_id, system_program_entry),
    ];

    let ok = [Check::success()];
    let rejected = [Check::err(solana_program_error::ProgramError::InvalidAccountData)];
    let chain: Vec<(&Instruction, &[Check])> = vec![
        (&instructions[0], &ok[..]),
        (&instructions[1], &ok[..]),
        (&instructions[2], &rejected[..]),
    ];

    mollusk.process_and_validate_instruction_chain(&chain, &accounts);
}
