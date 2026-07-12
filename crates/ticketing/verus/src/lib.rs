//! Pinocchio ticketing program (Verus lane): trusted imperative shells +
//! SMT-verified decision cores.
//!
//! The program is behavior-identical to `ticketing-creusot`/`ticketing-kani`;
//! the spec layer is Verus. The crate is split along the verification boundary:
//!
//! - [`mod@instructions`] - the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, persistence. Everything that touches the
//!   Solana runtime. Plain Rust outside `verus!{}`, so outside verification.
//! - `flows` - the decision cores: pure functions over wire-level state values,
//!   written inside `verus!{}` with their contracts inline, discharged by `just
//!   verus verify`.
//! - `domain` - state predicates (`spec fn`s), the verified typed mutators, and
//!   the external specs for what vstd does not cover (an opaque `Address`, two
//!   trusted `nz_*` wrappers - the residue of Creusot's axiom layer;
//!   `NonZeroU64` itself is modeled by vstd's `std_specs/nonzero`).
//!
//! The whole verified surface lives inside `verus!{}` blocks. The `verus!`
//! proc-macro erases every ghost item - specs, contracts, proofs - unless the
//! Verus driver is compiling (`--cfg verus_keep_ghost`), so ordinary
//! `cargo build` and `cargo build-sbf` expand the same source to the real
//! program (host+SBF) with zero verification overhead and no `vstd` in the
//! graph, while `cargo verus verify --features verus` checks the cores. Only
//! `vstd` and the foreign-type registration block (spec-only declarations) sit
//! behind the `verus` feature.

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod domain;
mod flows;
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
