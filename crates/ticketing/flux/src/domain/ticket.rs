//! Ticket constructor. The ticket invariant (event and buyer addresses set) is
//! carried as the `Ticket` type invariant; `ticket_new` is trusted, building
//! the opaque wire struct.
use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{
    Ticket,
    wire::{AccountHeader, Reserved},
};

/// Builds a buyer's ticket for a priced sale.
#[flux_rs::trusted]
#[flux_rs::sig(
    fn(
        event: Address[@ev],
        buyer: Address[@b],
        paid_lamports: NonZeroU64[@p],
        quantity: NonZeroU64[@q],
        bump: u8[@bp],
        created_slot: u64[@slot],
    ) -> Ticket[Ticket {
        event: ev,
        buyer: b,
        quantity: q,
        paid_lamports: p,
        bump: bp,
        created_slot: slot,
        updated_slot: slot,
    }]
    requires is_set(ev) && is_set(b)
)]
pub fn ticket_new(
    event: Address,
    buyer: Address,
    paid_lamports: NonZeroU64,
    quantity: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> Ticket {
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
