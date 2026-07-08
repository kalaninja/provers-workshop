//! Domain behavior for the ticketing program, implemented as extension traits
//! on the wire-level state types from `ticketing-interface`.
//!
//! The domain layer lives here rather than on the interface types so the wire
//! crate stays free of the Creusot verifier. Each `*Ops` trait pairs a
//! declaration (carrying the preconditions) with one implementation (carrying
//! the full contract); the traits are `pub(crate)` and pulled into scope by
//! the verified `flows` cores. The orphan rule allows these crate-local
//! traits to be implemented for the foreign state types.

mod address;
mod axioms;
mod event;
mod nonzero;
mod ticket;

pub(crate) use address::AddressOps;
#[cfg(creusot)]
pub(crate) use axioms::int_min;
pub(crate) use event::EventOps;
pub(crate) use nonzero::NonZeroOps;
pub(crate) use ticket::TicketOps;
