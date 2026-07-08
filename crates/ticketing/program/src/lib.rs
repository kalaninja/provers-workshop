//! Pinocchio ticketing program: trusted imperative shells + verified decision
//! cores.
//!
//! The crate is split along the verification boundary:
//!
//! - [`mod@instructions`] — the trusted imperative shells (`#[trusted]` under
//!   Creusot): account parsing, PDA creation, clock reads, CPIs, and
//!   persistence. Everything that touches the Solana runtime.
//! - `flows` — the verified decision cores: pure functions over wire-level
//!   state values, with Creusot `requires`/`ensures` contracts proved by `just
//!   creusot verify`.
//! - `domain` — predicates, invariants, and typed mutators on the wire state
//!   types, plus the trusted axioms modelling foreign types (`NonZeroU64`,
//!   `Address`).

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

#[cfg(creusot)]
use creusot_std::prelude::*;

mod domain;
mod flows;
#[cfg_attr(creusot, trusted)]
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
