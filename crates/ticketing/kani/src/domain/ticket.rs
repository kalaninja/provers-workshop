//! Construction and consistency predicates for a [`Ticket`].

use core::num::NonZeroU64;

use pinocchio::Address;
#[cfg(kani)]
use ticketing_interface::Event;
use ticketing_interface::{
    Ticket,
    wire::{AccountHeader, Reserved},
};

#[cfg(kani)]
use crate::domain::AddressOps;

/// Construction and consistency predicates for a [`Ticket`].
pub(crate) trait TicketOps {
    /// Ticket invariant: the event and buyer references are set.
    #[cfg(kani)]
    fn invariant(self) -> bool;

    /// The complete initial ticket state written by `new`: pinned fields and
    /// the invariant.
    #[cfg(kani)]
    fn is_initial(
        self,
        event: Address,
        buyer: Address,
        paid_lamports: NonZeroU64,
        quantity: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> bool;

    /// Cross-account relation to the event this ticket was sold for: the ticket
    /// points at the event and its quantity is covered by the event's sales
    /// counter.
    #[cfg(kani)]
    fn is_consistent_with(self, event_addr: Address, event: Event) -> bool;

    /// Builds a fresh ticket record. Expects non-default account references,
    /// established by the purchasing instruction flow.
    fn new(
        event: Address,
        buyer: Address,
        paid_lamports: NonZeroU64,
        quantity: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> Self
    where
        Self: Sized;
}

impl TicketOps for Ticket {
    #[cfg(kani)]
    fn invariant(self) -> bool {
        self.event.is_set() && self.buyer.is_set()
    }

    #[cfg(kani)]
    fn is_initial(
        self,
        event: Address,
        buyer: Address,
        paid_lamports: NonZeroU64,
        quantity: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> bool {
        self.event == event
            && self.buyer == buyer
            && self.paid_lamports == paid_lamports
            && self.quantity == quantity
            && self.bump == bump
            && self.created_slot == created_slot
            && self.updated_slot == created_slot
            && self.invariant()
    }

    #[cfg(kani)]
    fn is_consistent_with(self, event_addr: Address, event: Event) -> bool {
        self.event == event_addr && self.quantity.get() <= event.sold
    }

    fn new(
        event: Address,
        buyer: Address,
        paid_lamports: NonZeroU64,
        quantity: NonZeroU64,
        bump: u8,
        created_slot: u64,
    ) -> Self {
        Ticket {
            header: AccountHeader::new(),
            created_slot,
            updated_slot: created_slot,
            event,
            buyer,
            quantity,
            paid_lamports,
            bump,
            _reserved: Reserved::ZERO,
        }
    }
}
