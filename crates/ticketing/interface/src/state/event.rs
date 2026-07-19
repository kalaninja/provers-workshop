use core::num::NonZeroU64;

use bytemuck::{CheckedBitPattern, Contiguous, NoUninit};
use pinocchio::Address;
use ticketing_macros::{CodamaAccount, CodamaType, StateAccount};

use crate::{
    program::state::{CanonicalPda, PdaAccount, SlotTracked},
    wire::{AccountHeader, AccountHeaderSpec, BoundedString, Discriminator, Reserved},
};

/// Maximum byte length of an event name.
pub const MAX_EVENT_NAME_BYTES: usize = 64;

/// Fixed-capacity event name stored on the wire.
pub type EventName = BoundedString<MAX_EVENT_NAME_BYTES>;

/// Lifecycle states an `Event` account can occupy.
#[repr(u8)]
#[derive(
    Clone, Copy, CheckedBitPattern, Contiguous, Debug, Eq, NoUninit, PartialEq, CodamaType,
)]
pub enum EventStatus {
    /// Tickets may be purchased.
    SalesOpen = 0,
    /// Ticket sales are closed.
    SalesClosed = 1,
}

/// Root account for one ticketed event.
///
/// # Memory layout
///
/// Total size: 152 bytes, alignment: 8.
///
/// ```text
/// offset  size  field
/// ------  ----  -------------------------------------------------------------
///      0     8  header          (AccountHeader<Event>
///                                = discriminator[7] + version[1], align 1)
///      8     8  created_slot    (u64, align 8)
///     16     8  updated_slot    (u64, align 8)
///     24    32  organizer       (Address, align 1)
///     56     8  capacity        (NonZeroU64, align 8)
///     64     8  price_lamports  (NonZeroU64, align 8)
///     72     8  sold            (u64, align 8)
///     80    66  name            (BoundedString<64>, align 2)
///    146     1  bump            (u8, align 1)
///    147     1  status          (EventStatus, align 1)
///    148     4  _reserved       (align pad)
/// ```
///
/// `_reserved: Reserved<4>` is an alignment pad: the preceding fields sum
/// to 148 bytes, and the struct's alignment of 8 (inherited from `u64`
/// fields) rounds the size up to 152. Schema evolution happens through
/// `AccountHeader::VERSION` bumps, so this reserved region also doubles as
/// forward-compat space a future schema version can claim for real fields.
#[repr(C)]
#[derive(Clone, Copy, NoUninit, CheckedBitPattern, CodamaAccount, StateAccount, PartialEq, Eq)]
#[codama(discriminator(bytes = [84, 75, 84, 69, 86, 78, 84, 1], offset = 0))]
pub struct Event {
    /// Versioned account header: discriminator + schema version.
    #[codama(type = fixed_size(bytes, 8))]
    pub header: AccountHeader<Event>,
    /// Slot when the event account was created.
    pub created_slot: u64,
    /// Slot when the event account was last updated.
    pub updated_slot: u64,
    /// Organizer authority that created the event and receives payments.
    pub organizer: Address,
    /// Total number of tickets that can be sold. Must be non-zero.
    #[codama(type = number(u64))]
    pub capacity: NonZeroU64,
    /// Price of one ticket in lamports. Must be non-zero.
    #[codama(type = number(u64))]
    pub price_lamports: NonZeroU64,
    /// Number of tickets sold so far.
    pub sold: u64,
    /// Human-readable event name.
    pub name: EventName,
    /// Canonical event PDA bump.
    pub bump: u8,
    /// Current event lifecycle status.
    pub status: EventStatus,
    /// Alignment / forward-compat padding; must be all-zero.
    #[codama(type = fixed_size(bytes, 4))]
    pub _reserved: Reserved<4>,
}

const _: () = {
    assert!(size_of::<Event>() == 152);
    assert!(align_of::<Event>() == 8);
};

impl SlotTracked for Event {
    fn current_slot(&self) -> u64 {
        self.updated_slot
    }
    fn set_current_slot(&mut self, slot: u64) {
        self.updated_slot = slot;
    }
}

impl AccountHeaderSpec for Event {
    const DISCRIMINATOR: Discriminator = *b"TKTEVNT";
    const VERSION: u8 = 1;
}

impl CanonicalPda<1, 2> for Event {
    type Seeds = Address;
    const NAMESPACE: &'static [u8] = b"event";

    fn get_bump(&self) -> u8 {
        self.bump
    }

    fn get_seeds(&self) -> Address {
        self.organizer
    }

    fn variant_seeds(organizer: &Address) -> [&[u8]; 1] {
        [organizer.as_ref()]
    }
}

impl PdaAccount<1, 2, 3> for Event {}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use pinocchio::Address;

    use super::{Event, EventName, EventStatus};
    use crate::{
        program::state::StateAccount,
        wire::{AccountHeader, AccountHeaderSpec, Reserved},
    };

    fn nonzero(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test fixture is non-zero")
    }

    fn valid_event() -> Event {
        Event {
            header: AccountHeader::new(),
            created_slot: 42,
            updated_slot: 42,
            organizer: Address::from([1; 32]),
            capacity: nonzero(500),
            price_lamports: nonzero(2_000_000),
            sold: 0,
            name: EventName::from_bytes(b"rust-summit-2026").expect("name fits"),
            bump: 250,
            status: EventStatus::SalesOpen,
            _reserved: Reserved::ZERO,
        }
    }

    #[test]
    fn writes_event_state_layout() {
        let event = valid_event();
        let mut data = [0; Event::LEN];

        event.write_to(&mut data).expect("event state writes");

        let mut offset = 0;
        assert_eq!(&data[offset..offset + Event::DISCRIMINATOR.len()], &Event::DISCRIMINATOR);
        offset += Event::DISCRIMINATOR.len();
        assert_eq!(data[offset], Event::VERSION);
        offset += AccountHeader::<Event>::LEN - Event::DISCRIMINATOR.len();
        assert_eq!(&data[offset..offset + 8], &42_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 8], &42_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 32], &[1; 32]);
        offset += 32;
        assert_eq!(&data[offset..offset + 8], &500_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 8], &2_000_000_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 8], &0_u64.to_le_bytes());
        offset += 8;
        assert_eq!(&data[offset..offset + 2], &16_u16.to_le_bytes());
        offset += 2;
        assert_eq!(&data[offset..offset + 16], b"rust-summit-2026");
        offset += 16;
        assert_eq!(&data[offset..offset + 48], &[0; 48]);
        offset += 48;
        assert_eq!(data[offset], 250);
        offset += 1;
        assert_eq!(data[offset], EventStatus::SalesOpen as u8);
        offset += 1;
        assert_eq!(&data[offset..offset + 4], &[0; 4]);
        offset += 4;
        assert_eq!(offset, Event::LEN);
    }

    #[test]
    fn rejects_invalid_header_discriminator() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        data[..Event::DISCRIMINATOR.len()].copy_from_slice(b"BADEVNT");

        assert!(Event::cast(&data).is_err());
    }

    #[test]
    fn rejects_invalid_header_version() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        data[Event::DISCRIMINATOR.len()] = Event::VERSION + 1;

        assert!(Event::cast(&data).is_err());
    }

    #[test]
    fn cast_rejects_invalid_status_byte() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        let status_offset = core::mem::offset_of!(Event, status);
        data[status_offset] = 255;

        assert!(Event::cast(&data).is_err());
    }

    #[test]
    fn rejects_nonzero_reserved_bytes() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        let last = data.len() - 1;
        data[last] = 1;

        assert!(Event::cast(&data).is_err());
    }

    #[test]
    fn cast_rejects_zero_capacity() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        let offset = core::mem::offset_of!(Event, capacity);
        data[offset..offset + 8].fill(0);

        assert!(Event::cast(&data).is_err());
    }

    #[test]
    fn cast_rejects_zero_price() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        let offset = core::mem::offset_of!(Event, price_lamports);
        data[offset..offset + 8].fill(0);

        assert!(Event::cast(&data).is_err());
    }

    #[test]
    fn cast_rejects_oversized_name_len() {
        let event = valid_event();
        let mut data = [0; Event::LEN];
        event.write_to(&mut data).expect("event state writes");
        let offset = core::mem::offset_of!(Event, name);
        data[offset..offset + 2].copy_from_slice(&65_u16.to_le_bytes());

        assert!(Event::cast(&data).is_err());
    }
}
