//! Pinocchio ticketing program (Kani lane): trusted imperative shells +
//! bounded-model-checked decision cores.
//!
//! The program is byte-identical to `ticketing-creusot`; only the spec layer
//! differs. The crate is split along the verification boundary:
//!
//! - [`mod@instructions`] — the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, and persistence. Everything that touches the
//!   Solana runtime. Kani never enters them (no harness reaches them).
//! - `flows` — the decision cores: pure functions over wire-level state values.
//!   Their contracts are discharged by the `#[kani::proof_for_contract]`
//!   harnesses in [`mod@proofs`], run by `just kani verify`.
//! - `domain` — predicates, invariants, and typed mutators on the wire state
//!   types. Under Kani the foreign-type shims (`NonZeroU64`, `Address`) are
//!   concrete code the model checker verifies, not trusted axioms.

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod domain;
mod flows;
mod instructions;
#[cfg(kani)]
mod proofs;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
