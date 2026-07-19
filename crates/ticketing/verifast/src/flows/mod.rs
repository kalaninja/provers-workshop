//! Decision cores, one module per instruction: pure functions over wire-level
//! state values that make the instruction's decision and return the state for
//! the trusted shell to persist.
pub(crate) mod event_close;
pub(crate) mod event_create;
pub(crate) mod ticket_buy;
