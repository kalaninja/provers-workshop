//! Domain behavior for the ticketing program, implemented as extension traits
//! on the wire-level state types from `ticketing-interface`.
//!
//! The workshop keeps the program's behavior identical across prover crates but
//! lets each lane be written the way its tool is best used. Where the Creusot
//! lane routes the foreign `NonZeroU64`/`Address` operations through
//! `#[trusted]` axiom shims (`ax_*`, opaque `int`/`is_set`), the Kani lane
//! calls the real std methods directly (`checked_mul`, `min`, `==`, ...) and
//! gives `is_set` a concrete definition — concrete code the model checker
//! verifies, so nothing on the foreign types is trusted. The predicates are
//! ordinary `#[cfg(kani)]` boolean methods on the `*Ops` traits; the orphan
//! rule is why these are crate-local extension traits.

#[cfg(kani)]
mod address;
mod event;
mod ticket;

#[cfg(kani)]
pub(crate) use address::AddressOps;
pub(crate) use event::EventOps;
pub(crate) use ticket::TicketOps;
