//! Domain layer for the hax ticketing lane, split along the extraction
//! boundary:
//!
//! - [`mod@event`] - the pure accounting/lifecycle logic the flow cores call
//!   (clamp, pricing, status step). hax extracts it and its inline contracts
//!   are proved in Lean.
//! - [`mod@glue`] - the field-level bridges to the opaque wire types
//!   (`Event`/`Ticket`/`NonZeroU64`): accessors, setters, the `NonZero <-> u64`
//!   conversions, and the constructors. This is the trusted surface: each
//!   function carries a `#[hax_lib::lean::replace(...)]` that swaps its
//!   extraction for a concrete Lean model over the committed model preamble.
pub mod event;
pub mod glue;
