//! Construction of a [`Ticket`], as an extension trait on the foreign wire
//! type.
use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{
    Ticket,
    wire::{AccountHeader, Reserved},
};

/// Construction operations on a [`Ticket`].
pub(crate) trait TicketOps {
    /// Builds a fresh ticket record for a priced sale.
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
