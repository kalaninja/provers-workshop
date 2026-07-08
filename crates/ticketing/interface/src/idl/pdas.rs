//! Codama PDA definitions for the ticketing program.
//!
//! These unit structs exist only so codama emits each canonical PDA — with its
//! typed, ordered seeds — into the IDL's `program.pdas`. They mirror the
//! `CanonicalPda` impls in [`crate::state`] (namespace prepended to the
//! variant seeds). The `#[codama(name = ...)]` override gives each PDA the
//! canonical node name (matching its account) despite the distinct Rust struct
//! name. They are never instantiated at runtime.

// Read from the source AST by codama only; no Rust callers.
#![allow(dead_code)]

use ticketing_macros::CodamaPda;

/// Event PDA: `["event", organizer]`.
#[derive(CodamaPda)]
#[codama(name = "event")]
#[codama(seed(type = string(utf8), value = "event"))]
#[codama(seed(name = "organizer", type = public_key))]
struct EventPda;

/// Per-buyer ticket PDA: `["ticket", event, buyer]`.
#[derive(CodamaPda)]
#[codama(name = "ticket")]
#[codama(seed(type = string(utf8), value = "ticket"))]
#[codama(seed(name = "event", type = public_key))]
#[codama(seed(name = "buyer", type = public_key))]
struct TicketPda;
