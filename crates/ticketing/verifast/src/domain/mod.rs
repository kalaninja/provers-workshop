//! Domain layer: the accounting/lifecycle logic the flow cores call, as
//! extension traits on the foreign wire types (the orphan rule forbids inherent
//! methods on `Event`/`Ticket`).
mod event;
mod ticket;

pub(crate) use event::EventOps;
pub(crate) use ticket::TicketOps;
