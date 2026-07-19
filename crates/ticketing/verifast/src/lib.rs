//! Pinocchio ticketing program (VeriFast lane): trusted imperative shells +
//! pure decision cores.
//!
//! A port of the `ticketing-creusot` reference program (behavior-identical to
//! every lane) with no in-source spec layer and no verifier dependency - like
//! the Aeneas lane, ordinary `cargo build` and `cargo build-sbf` expand this to
//! the real program with zero verification overhead. The crate is split along
//! the verification boundary:
//!
//! - [`mod@instructions`] - the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, persistence. Everything that touches the
//!   Solana runtime.
//! - `flows` - the decision cores: pure functions over wire-level state values.
//! - `domain` - the accounting/lifecycle logic the cores call (clamp, pricing,
//!   status step).
//!
//! VeriFast cannot verify these cores directly: its Rust frontend cannot ingest
//! the crate's dependency graph (the external
//! pinocchio/bytemuck/ticketing-interface types). The lane's proof
//! is a self-contained, behaviorally-identical model of the three cores,
//! discharged by `just verifast verify`; see docs/verifast.md for where the
//! model lives and the cost.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod domain;
mod flows;
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
