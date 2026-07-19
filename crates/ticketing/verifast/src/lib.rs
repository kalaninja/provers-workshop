//! Pinocchio ticketing program (VeriFast lane): trusted imperative shells +
//! pure decision cores.
//!
//! A port of the `ticketing-creusot` reference program with the same wire
//! protocol and decision semantics, but no in-source spec layer or verifier
//! dependency. Like the Aeneas lane, ordinary `cargo build` and
//! `cargo build-sbf` compile the plain program. The crate is split along the
//! verification boundary:
//!
//! - `instructions` - the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, persistence. Everything that touches the
//!   Solana runtime.
//! - `flows` - the decision cores: pure functions over wire-level state values.
//! - `domain` - the accounting/lifecycle logic the cores call (clamp, pricing,
//!   status step).
//!
//! VeriFast cannot verify these cores directly: its Rust frontend cannot ingest
//! the crate's dependency graph (the external
//! pinocchio/bytemuck/ticketing-interface types). The lane's proof
//! is a self-contained model intended to reproduce the three cores' decision
//! semantics, discharged by `just verifast verify`; see docs/verifast.md for
//! where the model lives and the resulting trust boundary.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod domain;
mod flows;
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
