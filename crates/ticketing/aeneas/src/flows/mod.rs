//! Verified decision cores - the pure per-instruction decisions. Charon
//! extracts this module (and the `domain::event` accounting it calls) to LLBC;
//! Aeneas translates it to Lean, where the contracts are proved under
//! `verif/ticketing_aeneas`.
pub mod event_close;
pub mod event_create;
pub mod ticket_buy;
