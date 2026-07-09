//! Lifecycle and accounting operations on an [`Event`].

use core::num::NonZeroU64;

use pinocchio::{Address, ProgramResult, error::ProgramError};
use ticketing_interface::{
    Event, EventName, EventStatus,
    wire::{AccountHeader, Reserved},
};

#[cfg(kani)]
use crate::domain::AddressOps;

/// Legal event status steps: `SalesOpen -> SalesClosed`, plus each status to
/// itself. Every status is reachable from itself because a flow's no-op / error
/// path leaves the status unchanged and the transition guard is unconditional.
/// No regress.
#[cfg(kani)]
fn is_legal_event_status_transition(from: EventStatus, to: EventStatus) -> bool {
    from == to || (from == EventStatus::SalesOpen && to == EventStatus::SalesClosed)
}

/// Lifecycle and accounting operations on an [`Event`].
pub(crate) trait EventOps {
    /// Event invariant: the organizer is set, sold tickets never exceed
    /// capacity, and the worst-case payment (`capacity * price_lamports`) fits
    /// in `u64`.
    #[cfg(kani)]
    fn invariant(self) -> bool;

    /// Whether ticket sales are open.
    #[cfg(kani)]
    fn is_status_sales_open(self) -> bool;

    /// Whether ticket sales are closed.
    #[cfg(kani)]
    fn is_status_sales_closed(self) -> bool;

    /// The complete initial event state written by `new`: a zeroed sales
    /// counter and pinned fields, the invariant, and the sales-open status.
    #[cfg(kani)]
    fn is_initial(
        self,
        organizer: Address,
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8,
        created_slot: u64,
    ) -> bool;

    /// Whether `self` is a legal successor of `prev`: the immutable core fields
    /// are fixed, the sales total never decreases, and `prev.status ->
    /// self.status` is a legal step.
    #[cfg(kani)]
    fn is_legal_transition_from(self, prev: Self) -> bool
    where
        Self: Sized;

    /// Builds a fresh sales-open event with a zeroed sales counter — the base
    /// case for the invariant. Expects a non-default organizer and bounded
    /// economics, established by the creating instruction flow.
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

    /// Returns Ok when the supplied authority is this event's organizer.
    fn validate_organizer(&self, organizer: &Address) -> ProgramResult;

    /// Returns Ok when this event's ticket sales are open.
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
    #[cfg(kani)]
    fn invariant(self) -> bool {
        self.organizer.is_set()
            && self.sold <= self.capacity.get()
            && self.capacity.get().checked_mul(self.price_lamports.get()).is_some()
    }

    #[cfg(kani)]
    fn is_status_sales_open(self) -> bool {
        matches!(self.status, EventStatus::SalesOpen)
    }

    #[cfg(kani)]
    fn is_status_sales_closed(self) -> bool {
        matches!(self.status, EventStatus::SalesClosed)
    }

    #[cfg(kani)]
    fn is_initial(
        self,
        organizer: Address,
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8,
        created_slot: u64,
    ) -> bool {
        self.sold == 0
            && self.created_slot == created_slot
            && self.updated_slot == created_slot
            && self.organizer == organizer
            && self.capacity == capacity
            && self.price_lamports == price_lamports
            && self.name == name
            && self.bump == bump
            && self.invariant()
            && self.is_status_sales_open()
    }

    #[cfg(kani)]
    fn is_legal_transition_from(self, prev: Self) -> bool {
        self.header == prev.header
            && self.created_slot == prev.created_slot
            && self.organizer == prev.organizer
            && self.capacity == prev.capacity
            && self.price_lamports == prev.price_lamports
            && self.name == prev.name
            && self.bump == prev.bump
            && self._reserved == prev._reserved
            && self.sold >= prev.sold
            && is_legal_event_status_transition(prev.status, self.status)
    }

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
