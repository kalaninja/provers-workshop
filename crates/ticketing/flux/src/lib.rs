//! Pinocchio ticketing program (Flux lane): trusted imperative shells +
//! refinement-typed decision cores.
//!
//! Implements the same wire protocol and decision semantics as the
//! `ticketing-creusot` reference; only the spec layer differs. The crate is
//! split along the verification boundary:
//!
//! - `instructions` - the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, persistence. Everything that touches the
//!   Solana runtime. Each handler is `#[flux_rs::trusted]`, so Flux lifts its
//!   signature but never looks inside.
//! - `flows` - the decision cores: pure functions over wire-level state values,
//!   each carrying its contract as a `#[flux_rs::sig]` refinement signature,
//!   discharged by `just flux verify`.
//! - `domain` - the refinement vocabulary (`flux_rs::defs!` logic + the
//!   `extern_spec` registrations that refine the foreign wire types), the
//!   verified typed mutators, and the trusted numeric/wire wrappers - the
//!   boundary between the opaque wire/numeric models and checked arithmetic.
//!
//! `flux_rs` attributes strip to no-ops unless `cargo flux` is driving, so
//! ordinary `cargo build` and `cargo build-sbf` expand the same source to the
//! real program (host+SBF); verifier-only declarations are absent from the
//! target artifact. Unlike cargo-verus,
//! `cargo flux` sets `--cfg flux`, so the verification-only surface (the
//! `extern_spec` registrations and the `defs!` block) is `#[cfg(flux)]`-gated,
//! as under Kani.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![cfg_attr(flux, feature(nonzero_internals))]
#![cfg_attr(flux, allow(internal_features))]
#![allow(
    clippy::question_mark,
    reason = "Pinned Flux lacks the Try::branch refinement needed to track Result through `?`."
)]

mod domain;
mod flows;
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
