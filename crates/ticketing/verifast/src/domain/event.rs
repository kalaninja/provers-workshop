//! Lifecycle and accounting operations on an [`Event`], as an extension trait
//! (the orphan rule forbids inherent methods on the foreign wire type).
use core::num::NonZeroU64;

use pinocchio::{Address, ProgramResult, error::ProgramError};
use ticketing_interface::{
    Event, EventName, EventStatus,
    wire::{AccountHeader, Reserved},
};

/// Lifecycle and accounting operations on an [`Event`].
pub(crate) trait EventOps {
    /// Builds a fresh sales-open event with a zeroed sales counter.
    fn new(
        organizer: Address,
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8,
        created_slot: u64,
    ) -> Self
    where
        Self: Sized;

    /// Returns `Ok` when the supplied authority is this event's organizer.
    fn validate_organizer(&self, organizer: &Address) -> ProgramResult;

    /// Returns `Ok` when this event's ticket sales are open.
    fn require_sales_open(&self) -> ProgramResult;

    /// Records a ticket sale, clamping `requested` to remaining capacity and
    /// returning the quantity actually sold.
    fn record_sale(&mut self, requested: NonZeroU64) -> Result<NonZeroU64, ProgramError>;

    /// Computes the lamport payment due for `quantity` tickets at this event's
    /// unit price.
    fn payment_for(&self, quantity: NonZeroU64) -> Result<NonZeroU64, ProgramError>;

    /// Closes ticket sales.
    fn close_sales(&mut self) -> ProgramResult;
}

impl EventOps for Event {
    fn new(
        organizer: Address,
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8,
        created_slot: u64,
    ) -> Self {
        Event {
            header: AccountHeader::new(),
            created_slot,
            updated_slot: created_slot,
            organizer,
            capacity,
            price_lamports,
            sold: 0,
            name,
            bump,
            status: EventStatus::SalesOpen,
            _reserved: Reserved::ZERO,
        }
    }

    fn validate_organizer(&self, organizer: &Address) -> ProgramResult {
        if self.organizer != *organizer {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    #[allow(clippy::match_wildcard_for_single_variants)]
    fn require_sales_open(&self) -> ProgramResult {
        match self.status {
            EventStatus::SalesOpen => Ok(()),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn record_sale(&mut self, requested: NonZeroU64) -> Result<NonZeroU64, ProgramError> {
        self.require_sales_open()?;

        let remaining = self
            .capacity
            .get()
            .checked_sub(self.sold)
            .and_then(NonZeroU64::new)
            .ok_or(ProgramError::InvalidAccountData)?;

        let actual = remaining.min(requested);
        self.sold = self.sold.checked_add(actual.get()).ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(actual)
    }

    fn payment_for(&self, quantity: NonZeroU64) -> Result<NonZeroU64, ProgramError> {
        quantity.checked_mul(self.price_lamports).ok_or(ProgramError::ArithmeticOverflow)
    }

    fn close_sales(&mut self) -> ProgramResult {
        self.require_sales_open()?;
        self.status = EventStatus::SalesClosed;
        Ok(())
    }
}
