//! Construction and consistency predicates for a [`Ticket`].

use core::num::NonZeroU64;

#[cfg(creusot)]
use creusot_std::prelude::*;
use pinocchio::Address;
#[cfg(creusot)]
use ticketing_interface::Event;
use ticketing_interface::Ticket;
use ticketing_macros::creusot_extension;

use crate::domain::axioms::{account_header, zero_reserved};
#[cfg(creusot)]
use crate::domain::{AddressOps, NonZeroOps};

#[creusot_extension(trait TicketOps)]
/// Construction and consistency predicates for a [`Ticket`].
pub(crate) impl Ticket {
    /// Ticket invariant: the event and buyer references are set.
    #[cfg(creusot)]
    #[logic(open(crate), inline)]
    fn invariant(self) -> bool {
        pearlite! { self.event.is_set() && self.buyer.is_set() }
    }

    /// The complete initial ticket state written by [`TicketOps::new`]: pinned
    /// fields and the invariant ([`TicketOps::invariant`]).
    #[cfg(creusot)]
    #[logic(open(crate), inline)]
    fn is_initial(
        self,
        event: Address,
        buyer: Address,
        paid_lamports: NonZeroU64,
        quantity: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> bool {
        pearlite! {
            self.event == event
                && self.buyer == buyer
                && self.paid_lamports == paid_lamports
                && self.quantity == quantity
                && self.bump@ == bump@
                && self.created_slot@ == created_slot@
                && self.updated_slot@ == created_slot@
                && self.invariant()
        }
    }

    /// Cross-account relation to the event this ticket was sold for: the
    /// ticket points at the event and its quantity is covered by the event's
    /// sales counter.
    #[cfg(creusot)]
    #[logic(open(crate), inline)]
    fn is_consistent_with(self, event_addr: Address, event: Event) -> bool {
        pearlite! { self.event == event_addr && self.quantity.int() <= event.sold@ }
    }

    /// Builds a fresh ticket record. Expects non-default account references,
    /// established by the purchasing instruction flow.
    #[cfg_attr(creusot, requires(event.is_set() && buyer.is_set()))]
    #[cfg_attr(
        creusot,
        ensures(result.is_initial(event, buyer, paid_lamports, quantity, bump, created_slot))
    )]
    fn new(
        event: Address,
        buyer: Address,
        paid_lamports: NonZeroU64,
        quantity: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> Self
    where
        Self: Sized,
    {
        Ticket {
            header: account_header(),
            created_slot,
            updated_slot: created_slot,
            event,
            buyer,
            quantity,
            paid_lamports,
            bump,
            _reserved: zero_reserved(),
        }
    }
}
