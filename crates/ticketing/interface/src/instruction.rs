//! The ticketing program's instruction set.
//!
//! `TicketingInstruction` is the single source of truth for the program's wire
//! protocol. `#[derive(CodamaInstructions)]` together with the per-variant
//! `#[codama(account(...))]` metadata and `args` payloads drives codama IDL
//! generation. The on-chain dispatcher matches the raw discriminator byte
//! against the named `Self::*` discriminant constants and never builds this
//! enum. codama reads the `#[repr(u8)]` discriminants as the one-byte
//! instruction tags (these are not Anchor 8-byte discriminators).
//!
//! `#[derive(InstructionAccounts)]` generates a `<Variant>Accounts` struct +
//! `parse` per variant from the same `#[codama(account(...))]` table: exact
//! account count, `require_signer` for every `signer` slot, `require_writable`
//! for every `writable` slot, and an identity check for every slot whose
//! `default_value` pins a fixed program.

use ticketing_macros::{CodamaInstructions, InstructionAccounts};

use crate::{EventCreateArgs, TicketBuyArgs};

/// Every instruction accepted by the ticketing program, with its account list,
/// argument payload, and `#[repr(u8)]` wire discriminator.
///
/// The `#[codama(account(...))]` attributes on each variant are the single
/// source of truth for the wire account layout. `#[derive(CodamaInstructions)]`
/// consumes them to emit the IDL JSON; `#[derive(InstructionAccounts)]`
/// consumes the same table to generate the `<Variant>Accounts` structs +
/// `parse`, so the IDL and the runtime account extraction stay in sync by
/// construction.
#[derive(Debug, CodamaInstructions, InstructionAccounts)]
#[repr(u8)]
// This enum is the codama IDL + `InstructionAccounts` derive source; it is never
// built at runtime (the on-chain dispatcher matches the raw discriminator), so
// the variant-size disparity has no runtime cost. Boxing the large variant
// would break codama's IDL type resolution.
#[allow(clippy::large_enum_variant)]
// Keep the per-account attributes as a one-line-per-account table. rustfmt can
// reformat the simple ones but not those carrying `pda(...)`/`program(...)`
// (it can't parse them as meta), so without this it wraps only some — leaving
// an incoherent mix. `skip` keeps the whole table uniform; codama reads the
// tokens regardless of layout.
#[rustfmt::skip]
pub enum TicketingInstruction {
    /// Creates an event account with a fixed ticket supply and price.
    #[codama(account(name = "organizer", signer, writable, default_value = payer, docs = "Event organizer and rent payer"))]
    #[codama(account(name = "event", writable, default_value = pda("event", [account("organizer")]), docs = "Event PDA being initialized"))]
    #[codama(account(name = "system_program", default_value = program("system"), docs = "System program for the create-account CPI"))]
    EventCreate {
        /// Instruction arguments.
        args: EventCreateArgs,
    } = 0,

    /// Buys tickets for an event: pays the organizer in lamports and records
    /// the purchase in a per-buyer ticket account.
    #[codama(account(name = "buyer", signer, writable, default_value = payer, docs = "Ticket buyer; pays and funds the ticket rent"))]
    #[codama(account(name = "event", writable, docs = "Event with open ticket sales"))]
    #[codama(account(name = "organizer", writable, docs = "Receives the lamport payment; must match the event organizer"))]
    #[codama(account(name = "ticket", writable, default_value = pda("ticket", [account("event"), account("buyer")]), docs = "Ticket PDA created by this instruction"))]
    #[codama(account(name = "system_program", default_value = program("system"), docs = "System program for the create-account and transfer CPIs"))]
    TicketBuy {
        /// Instruction arguments.
        args: TicketBuyArgs,
    } = 1,

    /// Closes ticket sales for an event.
    #[codama(account(name = "organizer", signer, docs = "Event organizer"))]
    #[codama(account(name = "event", writable, docs = "Transitions SalesOpen to SalesClosed"))]
    EventClose = 2,
}

impl TicketingInstruction {
    // Named wire discriminators (the `#[repr(u8)]` tags), so the on-chain
    // dispatcher and `discriminator()` match on names rather than bare integer
    // literals. They mirror the enum's `= N` tags that codama reads into the IDL.
    /// Wire discriminator for [`TicketingInstruction::EventCreate`].
    pub const EVENT_CREATE: u8 = 0;
    /// Wire discriminator for [`TicketingInstruction::TicketBuy`].
    pub const TICKET_BUY: u8 = 1;
    /// Wire discriminator for [`TicketingInstruction::EventClose`].
    pub const EVENT_CLOSE: u8 = 2;

    /// Returns the one-byte wire discriminator (the `#[repr(u8)]` tag) for this
    /// instruction.
    #[must_use]
    pub const fn discriminator(&self) -> u8 {
        match self {
            Self::EventCreate { .. } => Self::EVENT_CREATE,
            Self::TicketBuy { .. } => Self::TICKET_BUY,
            Self::EventClose => Self::EVENT_CLOSE,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use super::TicketingInstruction;
    use crate::{EventCreateArgs, TicketBuyArgs, state::event::EventName};

    fn nonzero(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test fixture is non-zero")
    }

    fn create_args() -> EventCreateArgs {
        EventCreateArgs::new(
            nonzero(100),
            nonzero(1_000_000),
            EventName::from_bytes(b"expo").expect("name fits"),
        )
    }

    #[test]
    fn discriminator_routes_each_variant_to_its_named_constant() {
        // Spot-check; the Mollusk integration tests route every variant.
        assert_eq!(
            TicketingInstruction::EventCreate { args: create_args() }.discriminator(),
            TicketingInstruction::EVENT_CREATE,
        );
        assert_eq!(
            TicketingInstruction::TicketBuy { args: TicketBuyArgs::new(nonzero(1)) }
                .discriminator(),
            TicketingInstruction::TICKET_BUY,
        );
        assert_eq!(
            TicketingInstruction::EventClose.discriminator(),
            TicketingInstruction::EVENT_CLOSE,
        );
    }
}
