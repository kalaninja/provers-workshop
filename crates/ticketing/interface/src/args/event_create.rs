use core::num::NonZeroU64;

use bytemuck::{CheckedBitPattern, NoUninit};
use ticketing_macros::{CodamaType, InstructionArgs};

use crate::{state::event::EventName, wire::Reserved};

/// Wire-format arguments for the `event_create` instruction.
///
/// # Memory layout
///
/// Total size: 88 bytes, alignment: 8.
///
/// ```text
/// offset  size  field
/// ------  ----  ---------------------------------------------------
///      0     8  capacity        (NonZeroU64, align 8)
///      8     8  price_lamports  (NonZeroU64, align 8)
///     16    66  name            (BoundedString<64>, align 2)
///     82     6  _reserved       (align pad)
/// ```
///
/// The trailing `_reserved: Reserved<6>` is alignment padding: preceding
/// fields sum to 82 bytes, struct alignment of 8 rounds the size to 88.
/// Codama clients send the full 88-byte payload. The parser also accepts the
/// 82-byte form (omitting the pad) via `TRAILING_RESERVED = 6`, zero-filling
/// the missing bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, CheckedBitPattern, CodamaType, InstructionArgs)]
pub struct EventCreateArgs {
    /// Total number of tickets that can be sold. Must be non-zero.
    #[codama(type = number(u64))]
    pub capacity: NonZeroU64,
    /// Price of one ticket in lamports. Must be non-zero.
    #[codama(type = number(u64))]
    pub price_lamports: NonZeroU64,
    /// Human-readable event name.
    pub name: EventName,
    /// Alignment padding; must be all-zero.
    #[codama(type = fixed_size(bytes, 6))]
    _reserved: Reserved<6>,
}

const _: () = {
    assert!(size_of::<EventCreateArgs>() == 88);
    assert!(align_of::<EventCreateArgs>() == 8);
};

impl EventCreateArgs {
    /// Builds an `event_create` payload.
    #[must_use]
    pub fn new(capacity: NonZeroU64, price_lamports: NonZeroU64, name: EventName) -> Self {
        Self { capacity, price_lamports, name, _reserved: Reserved::ZERO }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use pinocchio::error::ProgramError;

    use super::EventCreateArgs;
    use crate::{program::instruction::InstructionArgs, state::event::EventName};

    fn nonzero(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test fixture is non-zero")
    }

    fn sample_args() -> EventCreateArgs {
        EventCreateArgs::new(
            nonzero(500),
            nonzero(2_000_000),
            EventName::from_bytes(b"rust-summit-2026").expect("name fits"),
        )
    }

    #[test]
    fn round_trips_full_length_payload() {
        let payload = bytemuck::bytes_of(&sample_args()).to_vec();
        assert_eq!(payload.len(), EventCreateArgs::LEN);

        let args = EventCreateArgs::parse(&payload).expect("payload parses");

        assert_eq!(args.capacity.get(), 500);
        assert_eq!(args.price_lamports.get(), 2_000_000);
        assert_eq!(args.name.as_bytes(), Ok(&b"rust-summit-2026"[..]));
    }

    #[test]
    fn parses_payload_without_trailing_reserved_bytes() {
        let full = bytemuck::bytes_of(&sample_args()).to_vec();
        let short = &full[..EventCreateArgs::LEN - EventCreateArgs::TRAILING_RESERVED];

        let args = EventCreateArgs::parse(short).expect("short payload parses");

        assert_eq!(args.capacity.get(), 500);
    }

    #[test]
    fn rejects_nonzero_reserved_tail() {
        let mut payload = bytemuck::bytes_of(&sample_args()).to_vec();
        let last = payload.len() - 1;
        payload[last] = 1;

        assert_eq!(
            EventCreateArgs::parse(&payload).err(),
            Some(ProgramError::InvalidInstructionData)
        );
    }

    #[test]
    fn rejects_zero_capacity_payload() {
        let mut payload = bytemuck::bytes_of(&sample_args()).to_vec();
        payload[..8].fill(0);

        assert_eq!(
            EventCreateArgs::parse(&payload).err(),
            Some(ProgramError::InvalidInstructionData)
        );
    }

    #[test]
    fn rejects_zero_price_payload() {
        let mut payload = bytemuck::bytes_of(&sample_args()).to_vec();
        payload[8..16].fill(0);

        assert_eq!(
            EventCreateArgs::parse(&payload).err(),
            Some(ProgramError::InvalidInstructionData)
        );
    }

    #[test]
    fn rejects_wrong_length_payload() {
        let payload = vec![0_u8; EventCreateArgs::LEN + 1];

        assert_eq!(
            EventCreateArgs::parse(&payload).err(),
            Some(ProgramError::InvalidInstructionData)
        );
    }
}
