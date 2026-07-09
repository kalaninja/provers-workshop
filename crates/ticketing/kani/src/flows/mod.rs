//! Decision cores, one module per instruction.
//!
//! Each core is a pure function over wire-level state values: it receives
//! destructured domain fields (never raw accounts or `*Args` structs), makes
//! the instruction's decision, and returns the state for the trusted shell to
//! persist. Their contracts — written in the domain predicates — are discharged
//! by the `#[kani::proof_for_contract]` harnesses in `crate::proofs`.

pub(crate) mod event_close;
pub(crate) mod event_create;
pub(crate) mod ticket_buy;
