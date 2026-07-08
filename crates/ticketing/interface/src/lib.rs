//! Wire-level interface of the ticketing program.
//!
//! Contains the on-chain account state types, instruction discriminator enum,
//! instruction argument structs, PDA derivation helpers, and layout constants.
//! Consumed by `ticketing-program` (the Pinocchio runtime handlers) and by any
//! off-chain client that needs the byte-level wire shapes.
//!
//! The [`wire`] module hosts framework-agnostic byte-layout primitives; the
//! [`program`] module hosts the Pinocchio program-building layer (account
//! validation, instruction-arg parsing, and PDA-backed state abstractions).

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

// Reads the program address from `[package.metadata.solana] program-id` in
// `Cargo.toml` at compile time — the single source codama reads too.
solana_address::declare_id!(solana_package_metadata::package_metadata!("solana.program-id"));

/// Instruction argument byte layouts.
pub mod args;
/// Codama IDL-only scaffolding (mirror types + PDA definitions) read only by
/// the IDL generator; never instantiated at runtime.
mod idl;
/// The ticketing program's instruction set: codama IDL source and wire
/// decoding.
pub mod instruction;
/// Pinocchio program-side helpers: account validation, instruction-arg
/// parsing, and PDA-backed state abstractions.
pub mod program;
/// On-chain account state layouts and PDA helpers.
pub mod state;
/// Mollusk-SVM integration-test fixtures shared across program and client test
/// suites. Behind the `test-utils` feature so its host-only deps stay out of
/// normal and SBF builds.
#[cfg(feature = "test-utils")]
pub mod test_utils;
/// Framework-agnostic wire-format primitives.
pub mod wire;

pub use args::{event_create::EventCreateArgs, ticket_buy::TicketBuyArgs};
pub use instruction::{
    EventCloseAccounts, EventCreateAccounts, TicketBuyAccounts, TicketingInstruction,
};
pub use state::{
    event::{Event, EventName, EventStatus, MAX_EVENT_NAME_BYTES},
    ticket::Ticket,
};
