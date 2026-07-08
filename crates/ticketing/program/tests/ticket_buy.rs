//! Integration tests for the `ticket_buy` instruction.

mod common;

use mollusk_svm::result::Check;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use ticketing_interface::{EventStatus, Ticket, TicketBuyArgs, program::state::StateAccount};

use crate::common::{
    DEFAULT_CAPACITY, DEFAULT_PRICE_LAMPORTS, PROGRAM_ID, WALLET_LAMPORTS, assert_program_error,
    derive_event_pda, derive_ticket_pda, empty_system_account, find_resulting_account,
    funded_system_account, mollusk, nonzero, parse_event_state, parse_ticket_state,
    rent_exempt_minimum, seeded_event_account, seeded_ticket_account, system_program_account,
    ticket_buy_instruction, ticket_buy_payload,
};

const ORGANIZER_LAMPORTS: u64 = 1_000_000;

struct BuyFixture {
    organizer: Pubkey,
    buyer: Pubkey,
    event_pda: Pubkey,
    ticket_pda: Pubkey,
}

fn fixture(tag: u8) -> BuyFixture {
    let organizer = Pubkey::new_from_array([tag; 32]);
    let buyer = Pubkey::new_from_array([tag + 100; 32]);
    let (event_pda, _) = derive_event_pda(&organizer);
    let (ticket_pda, _) = derive_ticket_pda(&event_pda, &buyer);
    BuyFixture { organizer, buyer, event_pda, ticket_pda }
}

#[test]
fn buys_tickets_and_pays_organizer_in_lamports() {
    let mollusk = mollusk();
    let f = fixture(1);
    let (_, expected_bump) = derive_ticket_pda(&f.event_pda, &f.buyer);
    let payment = 3 * DEFAULT_PRICE_LAMPORTS;

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &f.organizer,
        &f.ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(3))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        (f.ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::compute_units(7_916),
            Check::account(&f.organizer).lamports(ORGANIZER_LAMPORTS + payment).build(),
            Check::account(&f.buyer)
                .lamports(WALLET_LAMPORTS - payment - rent_exempt_minimum(Ticket::LEN))
                .build(),
        ],
    );

    let ticket_account =
        find_resulting_account(&result, &f.ticket_pda).expect("ticket account is created");
    let ticket = parse_ticket_state(&ticket_account.data);
    assert_eq!(ticket.event, f.event_pda);
    assert_eq!(ticket.buyer, f.buyer);
    assert_eq!(ticket.quantity.get(), 3);
    assert_eq!(ticket.paid_lamports.get(), payment);
    assert_eq!(ticket.bump, expected_bump);

    let event_account =
        find_resulting_account(&result, &f.event_pda).expect("event account persists");
    let event = parse_event_state(&event_account.data);
    assert_eq!(event.sold, 3);
    assert_eq!(event.status, EventStatus::SalesOpen);
}

#[test]
fn clamps_purchase_to_remaining_capacity() {
    let mollusk = mollusk();
    let f = fixture(2);
    let remaining = 2;
    let payment = remaining * DEFAULT_PRICE_LAMPORTS;

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &f.organizer,
        &f.ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(5))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                DEFAULT_CAPACITY - remaining,
                EventStatus::SalesOpen,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        (f.ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::compute_units(9_417),
            Check::account(&f.organizer).lamports(ORGANIZER_LAMPORTS + payment).build(),
        ],
    );

    let ticket_account =
        find_resulting_account(&result, &f.ticket_pda).expect("ticket account is created");
    let ticket = parse_ticket_state(&ticket_account.data);
    assert_eq!(ticket.quantity.get(), remaining);
    assert_eq!(ticket.paid_lamports.get(), payment);

    let event_account =
        find_resulting_account(&result, &f.event_pda).expect("event account persists");
    assert_eq!(parse_event_state(&event_account.data).sold, DEFAULT_CAPACITY);
}

#[test]
fn rejects_sold_out_event() {
    let mollusk = mollusk();
    let f = fixture(3);

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &f.organizer,
        &f.ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(1))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                DEFAULT_CAPACITY,
                EventStatus::SalesOpen,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        (f.ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidAccountData);
}

#[test]
fn rejects_purchase_after_sales_closed() {
    let mollusk = mollusk();
    let f = fixture(4);

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &f.organizer,
        &f.ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(1))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                10,
                EventStatus::SalesClosed,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        (f.ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidAccountData);
}

#[test]
fn rejects_payment_account_that_is_not_the_organizer() {
    let mollusk = mollusk();
    let f = fixture(5);
    let impostor = Pubkey::new_from_array([250; 32]);
    let (ticket_pda, _) = derive_ticket_pda(&f.event_pda, &f.buyer);

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &impostor,
        &ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(1))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
            ),
        ),
        (impostor, funded_system_account(ORGANIZER_LAMPORTS)),
        (ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidAccountData);
}

#[test]
fn rejects_repeat_purchase_for_same_buyer() {
    let mollusk = mollusk();
    let f = fixture(6);

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &f.organizer,
        &f.ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(1))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                2,
                EventStatus::SalesOpen,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        (f.ticket_pda, seeded_ticket_account(&f.event_pda, &f.buyer, 2, 4_000_000)),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    // `SystemError::AccountAlreadyInUse` surfaced by the CreateAccountAllowPrefund
    // CPI.
    assert_program_error(&result.program_result, ProgramError::Custom(0));
}

#[test]
fn creates_ticket_over_a_prefunded_slot() {
    // Anyone can send lamports to the derivable ticket PDA before the first
    // buy. CreateAccountAllowPrefund tolerates the prefunded *empty* slot - it
    // tops up only the rent deficit - so the purchase still creates the ticket.
    // A non-empty slot is still rejected (see
    // rejects_repeat_purchase_for_same_buyer).
    let mollusk = mollusk();
    let f = fixture(8);

    let ix = ticket_buy_instruction(
        &f.buyer,
        &f.event_pda,
        &f.organizer,
        &f.ticket_pda,
        ticket_buy_payload(&TicketBuyArgs::new(nonzero(3))),
    );
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        // Prefunded with 1 lamport, empty data, system-owned: the griefing shape.
        (f.ticket_pda, funded_system_account(1)),
        system_program_account(),
    ];

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::compute_units(13_916),
            Check::account(&f.ticket_pda)
                .owner(&PROGRAM_ID)
                .lamports(rent_exempt_minimum(Ticket::LEN))
                .build(),
        ],
    );
}

#[test]
fn rejects_zero_quantity_payload() {
    let mollusk = mollusk();
    let f = fixture(7);

    let mut payload = ticket_buy_payload(&TicketBuyArgs::new(nonzero(1)));
    payload[1..9].fill(0);

    let ix = ticket_buy_instruction(&f.buyer, &f.event_pda, &f.organizer, &f.ticket_pda, payload);
    let accounts = vec![
        (f.buyer, funded_system_account(WALLET_LAMPORTS)),
        (
            f.event_pda,
            seeded_event_account(
                &f.organizer,
                DEFAULT_CAPACITY,
                DEFAULT_PRICE_LAMPORTS,
                0,
                EventStatus::SalesOpen,
            ),
        ),
        (f.organizer, funded_system_account(ORGANIZER_LAMPORTS)),
        (f.ticket_pda, empty_system_account()),
        system_program_account(),
    ];

    let result = mollusk.process_instruction(&ix, &accounts);

    assert_program_error(&result.program_result, ProgramError::InvalidInstructionData);
}
