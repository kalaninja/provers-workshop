//! Verified decision cores - the pure per-instruction decisions. hax extracts
//! this module (and the `domain::event` accounting it calls) to Lean, where the
//! inline contract on each core is proved.
pub mod event_close;
pub mod event_create;
pub mod ticket_buy;
