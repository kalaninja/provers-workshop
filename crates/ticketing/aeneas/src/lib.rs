//! Pinocchio ticketing program (Aeneas lane): trusted imperative shells + pure
//! decision cores, proved externally in Lean.
//!
//! A port of the `ticketing-creusot` reference program (behavior-identical to
//! every lane) with the entire spec layer moved out of the source: no spec
//! attributes, no `cfg(aeneas)`, no verifier dependency - the contracts are
//! hand-written Lean under `verif/ticketing_aeneas`. The crate is split along
//! the verification boundary:
//!
//! - [`mod@instructions`] - the trusted imperative shells: account parsing, PDA
//!   creation, clock reads, CPIs, persistence. Everything that touches the
//!   Solana runtime. Charon never extracts these (`start_from =
//!   ["crate::flows"]`).
//! - `flows` - the decision cores: pure functions over wire-level state values.
//!   Charon extracts these to LLBC and Aeneas translates them to Lean. Written
//!   extraction-first: explicit `if`/`match` instead of `?`, which would drag
//!   the `Try`/`ControlFlow` trait machinery into the model.
//! - `domain` - the pure accounting/lifecycle logic the cores call
//!   ([`mod@domain::event`], extracted and proved) plus [`mod@domain::glue`],
//!   the opaque field-level bridges to `Event`/`Ticket`/`NonZeroU64` (the
//!   trusted surface, marked opaque to Charon and axiomatized in Lean).
//!
//! Aeneas adds nothing to the source, so ordinary `cargo build` and
//! `cargo build-sbf` expand this to the real program (host+SBF) with zero
//! verification overhead - the plainest program crate in the roster.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod domain;
mod flows;
mod instructions;

pub use instructions::process_instruction;

#[cfg(feature = "bpf-entrypoint")]
pinocchio::entrypoint!(process_instruction);
