//! Parity between the generated client and `ticketing-interface`, the source
//! of truth for the ticketing program's wire protocol.
//!
//! Each test either builds an instruction with the client encoder and parses
//! the bytes with the interface's `<Args>::parse` (the same parse the program's
//! dispatcher runs) after checking the discriminator, or checks that the client
//! derives the same PDAs as the interface; a regenerated client that drifts
//! from the interface fails here.

use solana_address::Address;
use ticketing_client::{
    TICKETING_ID, accounts,
    instructions::{
        EVENT_CLOSE_DISCRIMINATOR, EVENT_CREATE_DISCRIMINATOR, EventClose, EventCreate,
        EventCreateInstructionArgs, TICKET_BUY_DISCRIMINATOR, TicketBuy, TicketBuyInstructionArgs,
    },
    types::EventName,
};
use ticketing_interface::{
    Event, EventCreateArgs, ID, Ticket, TicketBuyArgs,
    program::{
        instruction::{InstructionArgs, NoArgs},
        state::CanonicalPda,
    },
};

fn event_name(bytes: &[u8]) -> EventName {
    let mut storage = [0u8; 64];
    storage[..bytes.len()].copy_from_slice(bytes);
    EventName { len: u16::try_from(bytes.len()).expect("name fits"), bytes: storage }
}

#[test]
fn program_id_matches_interface() {
    assert_eq!(TICKETING_ID, ID, "client program ID drifted from the interface's declared ID");
}

#[test]
fn event_create_encoding_round_trips_through_the_program_decoder() {
    // Account values are arbitrary; the parity check only reads the
    // instruction data.
    let instruction = EventCreate {
        organizer: Address::from([1; 32]),
        event: Address::from([2; 32]),
        system_program: Address::from([3; 32]),
    }
    .instruction(EventCreateInstructionArgs {
        capacity: 500,
        price_lamports: 2_000_000,
        name: event_name(b"rust-summit-2026"),
        reserved: [0; 6],
    });

    assert_eq!(instruction.program_id, ID, "builder targets the interface program ID");
    assert_eq!(EVENT_CREATE_DISCRIMINATOR, 0);

    let (discriminator, args) =
        instruction.data.split_first().expect("instruction data is non-empty");
    assert_eq!(*discriminator, EVENT_CREATE_DISCRIMINATOR);
    let args = EventCreateArgs::parse(args).expect("client-built args parse via the interface");
    assert_eq!(args.capacity.get(), 500, "capacity lost in encode/decode");
    assert_eq!(args.price_lamports.get(), 2_000_000, "price lost in encode/decode");
    assert_eq!(args.name.as_bytes(), Ok(&b"rust-summit-2026"[..]), "name lost in encode/decode");
}

#[test]
fn ticket_buy_encoding_round_trips_through_the_program_decoder() {
    let instruction = TicketBuy {
        buyer: Address::from([1; 32]),
        event: Address::from([2; 32]),
        organizer: Address::from([3; 32]),
        ticket: Address::from([4; 32]),
        system_program: Address::from([5; 32]),
    }
    .instruction(TicketBuyInstructionArgs { quantity: 42 });

    assert_eq!(TICKET_BUY_DISCRIMINATOR, 1);

    let (discriminator, args) =
        instruction.data.split_first().expect("instruction data is non-empty");
    assert_eq!(*discriminator, TICKET_BUY_DISCRIMINATOR);
    let args = TicketBuyArgs::parse(args).expect("client-built args parse via the interface");
    assert_eq!(args.quantity.get(), 42, "argument lost in encode/decode");
}

#[test]
fn event_close_encoding_round_trips_through_the_program_decoder() {
    let instruction =
        EventClose { organizer: Address::from([1; 32]), event: Address::from([2; 32]) }
            .instruction();

    assert_eq!(EVENT_CLOSE_DISCRIMINATOR, 2);
    let (discriminator, args) =
        instruction.data.split_first().expect("instruction data is non-empty");
    assert_eq!(*discriminator, EVENT_CLOSE_DISCRIMINATOR);
    NoArgs::parse(args).expect("client-built event_close carries no args");
}

#[test]
fn account_pdas_match_interface_derivation() {
    let organizer = Address::from([10; 32]);
    let event = Address::from([11; 32]);
    let buyer = Address::from([12; 32]);

    assert_eq!(
        accounts::Event::find_pda(&organizer).0,
        Event::derive_address(&ID, &organizer).expect("event pda").0,
    );
    assert_eq!(
        accounts::Ticket::find_pda(&event, &buyer).0,
        Ticket::derive_address(&ID, &(event, buyer)).expect("ticket pda").0,
    );
}
