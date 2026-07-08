use core::num::NonZeroU64;

use bytemuck::{CheckedBitPattern, NoUninit};
use ticketing_macros::{CodamaType, InstructionArgs};

/// Wire-format arguments for the `ticket_buy` instruction.
///
/// `quantity` is the maximum number of admissions the buyer is willing to
/// acquire. The handler clamps to remaining capacity and charges only for
/// what is actually sold.
#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, CheckedBitPattern, CodamaType, InstructionArgs)]
pub struct TicketBuyArgs {
    /// Maximum number of admissions the buyer is willing to acquire in this
    /// purchase. The handler clamps to remaining capacity. Must be non-zero.
    #[codama(type = number(u64))]
    pub quantity: NonZeroU64,
}

const _: () = {
    assert!(size_of::<TicketBuyArgs>() == 8);
    assert!(align_of::<TicketBuyArgs>() == 8);
};

impl TicketBuyArgs {
    /// Builds a purchase payload requesting up to `quantity` admissions.
    #[must_use]
    pub fn new(quantity: NonZeroU64) -> Self {
        Self { quantity }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use pinocchio::error::ProgramError;

    use super::TicketBuyArgs;
    use crate::program::instruction::InstructionArgs;

    fn nonzero(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test fixture is non-zero")
    }

    #[test]
    fn round_trips_quantity() {
        let payload = bytemuck::bytes_of(&TicketBuyArgs::new(nonzero(42))).to_vec();

        let args = TicketBuyArgs::parse(&payload).expect("payload parses");

        assert_eq!(args.quantity.get(), 42);
    }

    #[test]
    fn rejects_trailing_payload_bytes() {
        let mut payload = bytemuck::bytes_of(&TicketBuyArgs::new(nonzero(1))).to_vec();
        payload.push(0);

        let result = TicketBuyArgs::parse(&payload);

        assert_eq!(result.err(), Some(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn rejects_zero_quantity_payload() {
        let payload = vec![0_u8; TicketBuyArgs::LEN];

        let result = TicketBuyArgs::parse(&payload);

        assert_eq!(result.err(), Some(ProgramError::InvalidInstructionData));
    }
}
