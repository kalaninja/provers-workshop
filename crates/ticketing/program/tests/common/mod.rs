//! Shared fixtures and helpers for `ticketing-program` integration tests.
//!
//! The program is exercised through `mollusk-svm`, which loads the SBF `.so`
//! built by `cargo build-sbf`. Tests build wire payloads via constructor
//! methods (e.g. `EventCreateArgs::new`) and assert post-CPI state by
//! re-casting account bytes to typed state structs (e.g. `Event`).

#![allow(dead_code)]

use core::num::NonZeroU64;

use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
pub use ticketing_interface::test_utils::*;
use ticketing_interface::{
    Event, EventCreateArgs, EventName, EventStatus, Ticket, TicketBuyArgs,
    program::state::{CanonicalPda, StateAccount},
    wire::{AccountHeader, Reserved},
};

/// Program id under test, as declared by the interface crate.
pub const PROGRAM_ID: Pubkey = ticketing_interface::ID;
/// SBF artefact name (`<crate_name>.so`) produced by `cargo build-sbf`.
pub const PROGRAM_NAME: &str = "ticketing_program";

/// Default event capacity used by fixtures.
pub const DEFAULT_CAPACITY: u64 = 500;
/// Default ticket price used by fixtures.
pub const DEFAULT_PRICE_LAMPORTS: u64 = 2_000_000;

pub fn mollusk() -> Mollusk {
    mollusk_for(&PROGRAM_ID, PROGRAM_NAME)
}

pub fn nonzero(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("test fixture is non-zero")
}

pub fn derive_event_pda(organizer: &Pubkey) -> (Pubkey, u8) {
    Event::derive_address(&PROGRAM_ID, organizer).expect("event PDA derives")
}

pub fn derive_ticket_pda(event: &Pubkey, buyer: &Pubkey) -> (Pubkey, u8) {
    Ticket::derive_address(&PROGRAM_ID, &(*event, *buyer)).expect("ticket PDA derives")
}

/// Default event name fixture.
pub fn event_name() -> EventName {
    EventName::from_bytes(b"rust-summit-2026").expect("name fits")
}

/// Wire payload for `event_create` (tag 0 + args bytes).
pub fn event_create_payload(args: &EventCreateArgs) -> Vec<u8> {
    let mut data = vec![0_u8];
    data.extend_from_slice(bytemuck::bytes_of(args));
    data
}

/// Wire payload for `ticket_buy` (tag 1 + args bytes).
pub fn ticket_buy_payload(args: &TicketBuyArgs) -> Vec<u8> {
    let mut data = vec![1_u8];
    data.extend_from_slice(bytemuck::bytes_of(args));
    data
}

/// Wire payload for `event_close` (tag 2, no args).
pub fn event_close_payload() -> Vec<u8> {
    vec![2_u8]
}

pub fn event_create_instruction(organizer: &Pubkey, event: &Pubkey, data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*organizer, true),
            AccountMeta::new(*event, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn ticket_buy_instruction(
    buyer: &Pubkey,
    event: &Pubkey,
    organizer: &Pubkey,
    ticket: &Pubkey,
    data: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*buyer, true),
            AccountMeta::new(*event, false),
            AccountMeta::new(*organizer, false),
            AccountMeta::new(*ticket, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn event_close_instruction(organizer: &Pubkey, event: &Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*organizer, true),
            AccountMeta::new(*event, false),
        ],
        data: event_close_payload(),
    }
}

/// Pre-seeds a program-owned event account at the canonical PDA with the given
/// accounting state.
pub fn seeded_event_account(
    organizer: &Pubkey,
    capacity: u64,
    price_lamports: u64,
    sold: u64,
    status: EventStatus,
) -> Account {
    seeded_event_account_updated_at(organizer, capacity, price_lamports, sold, status, 0)
}

/// As [`seeded_event_account`], but with an explicit `updated_slot` — for
/// exercising the monotonic-slot guard.
pub fn seeded_event_account_updated_at(
    organizer: &Pubkey,
    capacity: u64,
    price_lamports: u64,
    sold: u64,
    status: EventStatus,
    updated_slot: u64,
) -> Account {
    let (_, bump) = derive_event_pda(organizer);
    let event = Event {
        header: AccountHeader::new(),
        created_slot: 0,
        updated_slot,
        organizer: *organizer,
        capacity: nonzero(capacity),
        price_lamports: nonzero(price_lamports),
        sold,
        name: event_name(),
        bump,
        status,
        _reserved: Reserved::ZERO,
    };
    account_from_state(bytemuck::bytes_of(&event))
}

/// Pre-seeds a program-owned ticket account at the canonical PDA.
pub fn seeded_ticket_account(event: &Pubkey, buyer: &Pubkey, quantity: u64, paid: u64) -> Account {
    let (_, bump) = derive_ticket_pda(event, buyer);
    let ticket = Ticket {
        header: AccountHeader::new(),
        created_slot: 0,
        updated_slot: 0,
        event: *event,
        buyer: *buyer,
        quantity: nonzero(quantity),
        paid_lamports: nonzero(paid),
        bump,
        _reserved: Reserved::ZERO,
    };
    account_from_state(bytemuck::bytes_of(&ticket))
}

fn account_from_state(data: &[u8]) -> Account {
    Account {
        lamports: rent_exempt_minimum(data.len()),
        data: data.to_vec(),
        owner: PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Re-casts raw account bytes into a typed [`Event`].
pub fn parse_event_state(data: &[u8]) -> Event {
    *Event::cast(data).expect("event state casts")
}

/// Re-casts raw account bytes into a typed [`Ticket`].
pub fn parse_ticket_state(data: &[u8]) -> Ticket {
    *Ticket::cast(data).expect("ticket state casts")
}
