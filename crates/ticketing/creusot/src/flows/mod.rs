//! Verified decision cores, one module per instruction.
//!
//! Each core is a pure function over wire-level state values: it receives
//! destructured domain fields (never raw accounts or `*Args` structs), makes
//! the instruction's decision, and returns the state for the trusted shell to
//! persist. The Creusot contracts on these functions — together with the
//! domain predicates they are written in — are the program's formal
//! specification, proved by `just creusot verify`.

pub(crate) mod event_close;
pub(crate) mod event_create;
pub(crate) mod ticket_buy;
