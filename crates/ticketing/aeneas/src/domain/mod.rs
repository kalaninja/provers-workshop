//! Domain layer for the Aeneas ticketing lane, split along the extraction
//! boundary:
//!
//! - [`mod@event`] - the pure accounting/lifecycle logic the flow cores call
//!   (clamp, pricing, status step). Charon extracts it; Aeneas proves its
//!   contracts in Lean.
//! - [`mod@glue`] - the field-level bridges to the opaque wire types
//!   (`Event`/`Ticket`/`NonZeroU64`): accessors, setters, the `NonZero <-> u64`
//!   conversions, and the constructors. This is the trusted surface: marked
//!   opaque to Charon (`[package.metadata.charon] opaque`) and axiomatized with
//!   `@[step]` specs in the Lean proofs.
pub mod event;
pub mod glue;
