use core::num::NonZeroU64;

use bytemuck::{CheckedBitPattern, NoUninit};
use pinocchio::Address;
use ticketing_macros::{CodamaAccount, StateAccount};

use crate::{
    program::state::{CanonicalPda, PdaAccount, SlotTracked},
    wire::{AccountHeader, AccountHeaderSpec, Discriminator, Reserved},
};

/// Per-buyer ticket record for one event.
///
/// One ticket account exists per `(event, buyer)` pair; a single purchase may
/// cover multiple admissions (`quantity`). The account address is the
/// canonical PDA derived from the event and buyer, so its existence is the
/// "already bought" signal — repeat purchases fail because the create CPI in
/// `ticket_buy` rejects an already-initialized account.
///
/// # Memory layout
///
/// Total size: 112 bytes, alignment: 8.
///
/// ```text
/// offset  size  field
/// ------  ----  -------------------------------------------------------------
///      0     8  header          (AccountHeader<Ticket>
///                                = discriminator[7] + version[1], align 1)
///      8     8  created_slot    (u64, align 8)
///     16     8  updated_slot    (u64, align 8)
///     24    32  event           (Address, align 1)
///     56    32  buyer           (Address, align 1)
///     88     8  quantity        (NonZeroU64, align 8)
///     96     8  paid_lamports   (NonZeroU64, align 8)
///    104     1  bump            (u8, align 1)
///    105     7  _reserved       (align pad)
/// ```
///
/// `_reserved: Reserved<7>` is an alignment pad: the preceding fields sum
/// to 105 bytes, and the struct's alignment of 8 (inherited from `u64`
/// fields) rounds the size up to 112. Schema evolution happens through
/// `AccountHeader::VERSION` bumps.
#[repr(C)]
#[derive(Clone, Copy, NoUninit, CheckedBitPattern, CodamaAccount, StateAccount, PartialEq, Eq)]
#[codama(discriminator(bytes = [84, 75, 84, 84, 67, 75, 84, 1], offset = 0))]
pub struct Ticket {
    /// Versioned account header: discriminator + schema version.
    #[codama(type = fixed_size(bytes, 8))]
    pub header: AccountHeader<Ticket>,
    /// Slot when the ticket account was created.
    pub created_slot: u64,
    /// Slot when the ticket account was last updated.
    pub updated_slot: u64,
    /// Event this ticket belongs to.
    pub event: Address,
    /// Buyer that paid for this ticket.
    pub buyer: Address,
    /// Number of admissions purchased. Must be non-zero.
    #[codama(type = number(u64))]
    pub quantity: NonZeroU64,
    /// Lamports paid for the ticket (`quantity * price_lamports`). Must be
    /// non-zero.
    #[codama(type = number(u64))]
    pub paid_lamports: NonZeroU64,
    /// Canonical ticket PDA bump.
    pub bump: u8,
    /// Alignment / forward-compat padding; must be all-zero.
    #[codama(type = fixed_size(bytes, 7))]
    pub _reserved: Reserved<7>,
}

const _: () = {
    assert!(size_of::<Ticket>() == 112);
    assert!(align_of::<Ticket>() == 8);
};

impl SlotTracked for Ticket {
    fn current_slot(&self) -> u64 {
        self.updated_slot
    }
    fn set_current_slot(&mut self, slot: u64) {
        self.updated_slot = slot;
    }
}

impl AccountHeaderSpec for Ticket {
    const DISCRIMINATOR: Discriminator = *b"TKTTCKT";
    const VERSION: u8 = 1;
}

impl CanonicalPda<2, 3> for Ticket {
    type Seeds = (Address, Address);
    const NAMESPACE: &'static [u8] = b"ticket";

    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn get_seeds(&self) -> (Address, Address) {
        (self.event, self.buyer)
    }

    fn variant_seeds((event, buyer): &(Address, Address)) -> [&[u8]; 2] {
        [event.as_ref(), buyer.as_ref()]
    }
}

impl PdaAccount<2, 3, 4> for Ticket {}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use pinocchio::Address;

    use super::Ticket;
    use crate::{
        program::state::StateAccount,
        wire::{AccountHeader, AccountHeaderSpec, Reserved},
    };

    fn nonzero(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test fixture is non-zero")
    }

    fn valid_ticket() -> Ticket {
        Ticket {
            header: AccountHeader::new(),
            created_slot: 42,
            updated_slot: 42,
            event: Address::from([1; 32]),
            buyer: Address::from([2; 32]),
            quantity: nonzero(3),
            paid_lamports: nonzero(6_000_000),
            bump: 250,
            _reserved: Reserved::ZERO,
        }
    }

    #[test]
    fn writes_ticket_state_layout() {
        let ticket = valid_ticket();
        let mut data = [0; Ticket::LEN];

        ticket.write_to(&mut data).expect("ticket state writes");

        let mut offset = 0;
        assert_eq!(&data[offset..offset + Ticket::DISCRIMINATOR.len()], &Ticket::DISCRIMINATOR);
        offset += Ticket::DISCRIMINATOR.len();
        assert_eq!(data[offset], Ticket::VERSION);
        offset += AccountHeader::<Ticket>::LEN - Ticket::DISCRIMINATOR.len();
        assert_eq!(&data[offset..offset + 8], &42_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 8], &42_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 32], &[1; 32]);
        offset += 32;
        assert_eq!(&data[offset..offset + 32], &[2; 32]);
        offset += 32;
        assert_eq!(&data[offset..offset + 8], &3_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 8], &6_000_000_u64.to_le_bytes());
        offset += 8;
        assert_eq!(data[offset], 250);
        offset += 1;
        assert_eq!(&data[offset..offset + 7], &[0; 7]);
        offset += 7;
        assert_eq!(offset, Ticket::LEN);
    }

    #[test]
    fn rejects_invalid_header_discriminator() {
        let ticket = valid_ticket();
        let mut data = [0; Ticket::LEN];
        ticket.write_to(&mut data).expect("ticket state writes");
        data[..Ticket::DISCRIMINATOR.len()].copy_from_slice(b"BADTCKT");

        assert!(Ticket::cast(&data).is_err());
    }

    #[test]
    fn rejects_invalid_header_version() {
        let ticket = valid_ticket();
        let mut data = [0; Ticket::LEN];
        ticket.write_to(&mut data).expect("ticket state writes");
        data[Ticket::DISCRIMINATOR.len()] = Ticket::VERSION + 1;

        assert!(Ticket::cast(&data).is_err());
    }

    #[test]
    fn rejects_nonzero_reserved_bytes() {
        let ticket = valid_ticket();
        let mut data = [0; Ticket::LEN];
        ticket.write_to(&mut data).expect("ticket state writes");
        let last = data.len() - 1;
        data[last] = 1;

        assert!(Ticket::cast(&data).is_err());
    }

    #[test]
    fn cast_rejects_zero_paid_lamports() {
        let ticket = valid_ticket();
        let mut data = [0; Ticket::LEN];
        ticket.write_to(&mut data).expect("ticket state writes");
        let offset = core::mem::offset_of!(Ticket, paid_lamports);
        data[offset..offset + 8].fill(0);

        assert!(Ticket::cast(&data).is_err());
    }

    #[test]
    fn cast_rejects_zero_quantity() {
        let ticket = valid_ticket();
        let mut data = [0; Ticket::LEN];
        ticket.write_to(&mut data).expect("ticket state writes");
        let offset = core::mem::offset_of!(Ticket, quantity);
        data[offset..offset + 8].fill(0);

        assert!(Ticket::cast(&data).is_err());
    }
}
