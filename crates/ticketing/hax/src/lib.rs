//! Pinocchio ticketing program (hax lane): trusted imperative shells + pure
//! decision cores, proved in Lean via hax's Rust -> Lean extraction.
//!
//! A port of the `ticketing-creusot` reference program (behavior-identical to
//! every lane). Unlike Aeneas - the other translate-to-Lean lane, which proves
//! from the outside with no in-source spec - hax carries its contracts *inline*
//! as `hax_lib::lean::after` attributes: embedded Lean Hoare-triple theorems
//! proved next to the code they specify, all `cfg(hax)`-gated so ordinary and
//! SBF builds compile the identical program with zero verification overhead.
//! The crate is split along the verification boundary:
//!
//! - [`mod@instructions`] - the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, persistence. Everything that touches the
//!   Solana runtime. Excluded from hax extraction.
//! - `flows` - the decision cores: pure functions over wire-level state values.
//!   hax extracts these to Lean and their inline contracts are proved. Written
//!   extraction-first: explicit `if`/`match` instead of `?`, which would drag
//!   the `Try`/`ControlFlow` trait machinery into the model.
//! - `domain` - the pure accounting/lifecycle logic the cores call
//!   ([`mod@domain::event`], extracted and proved) plus [`mod@domain::glue`],
//!   the field-level bridges to `Event`/`Ticket`/`NonZeroU64` (the trusted
//!   surface, whose extraction `hax_lib::lean::replace` swaps for concrete
//!   trusted Lean models).
//!
//! hax adds only `cfg(hax)`-gated attributes, so ordinary `cargo build` and
//! `cargo build-sbf` expand this to the real program (host+SBF) unchanged.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod domain;
mod flows;
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
