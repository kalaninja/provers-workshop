//! Lifecycle and accounting operations on an [`Event`].

use core::num::NonZeroU64;

#[cfg(creusot)]
use creusot_std::prelude::*;
use pinocchio::{Address, ProgramResult, error::ProgramError};
use ticketing_interface::{Event, EventName, EventStatus};
use ticketing_macros::creusot_extension;

#[cfg(creusot)]
use crate::domain::int_min;
use crate::domain::{
    AddressOps, NonZeroOps,
    axioms::{account_header, zero_reserved},
};

/// Legal event status steps: `SalesOpen -> SalesClosed`, plus each status to
/// itself. Every status is reachable from itself because a flow's no-op /
/// error path leaves the status unchanged and the transition guard is
/// unconditional. No regress. Consumed by
/// [`EventOps::is_legal_transition_from`].
#[cfg(creusot)]
#[logic(open(crate), inline)]
pub(crate) fn is_legal_event_status_transition(from: EventStatus, to: EventStatus) -> bool {
    pearlite! {
        from == to || (from == EventStatus::SalesOpen && to == EventStatus::SalesClosed)
    }
}

#[creusot_extension(trait EventOps)]
/// Lifecycle and accounting operations on an [`Event`].
pub(crate) impl Event {
    /// Event invariant: the organizer is set, sold tickets never exceed
    /// capacity, and the worst-case payment (`capacity * price_lamports`)
    /// fits in `u64`.
    #[cfg(creusot)]
    #[logic(open(crate), inline)]
    fn invariant(self) -> bool {
        pearlite! {
            self.organizer.is_set()
                && self.sold@ <= self.capacity.int()
                && self.capacity.int() * self.price_lamports.int() <= u64::MAX@
        }
    }

    /// Whether ticket sales are open.
    #[cfg(creusot)]
    #[logic(open, inline)]
    fn is_status_sales_open(self) -> bool {
        pearlite! { match self.status { EventStatus::SalesOpen => true, _ => false } }
    }

    /// Whether ticket sales are closed.
    #[cfg(creusot)]
    #[logic(open, inline)]
    fn is_status_sales_closed(self) -> bool {
        pearlite! { match self.status { EventStatus::SalesClosed => true, _ => false } }
    }

    /// The complete initial event state written by [`EventOps::new`]: a zeroed
    /// sales counter and pinned fields, the invariant
    /// (`EventOps::invariant`), and the sales-open status.
    #[cfg(creusot)]
    #[logic(open(crate), inline)]
    fn is_initial(
        self,
        organizer: Address,
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8,
        created_slot: u64,
    ) -> bool {
        pearlite! {
            self.sold@ == 0
                && self.created_slot@ == created_slot@
                && self.updated_slot@ == created_slot@
                && self.organizer == organizer
                && self.capacity == capacity
                && self.price_lamports == price_lamports
                && self.name == name
                && self.bump@ == bump@
                && self.invariant()
                && self.is_status_sales_open()
        }
    }

    /// Whether `self` is a legal successor of `prev` - the event's transition
    /// rules, asserted by every flow core that takes it `&mut` (so a too-wide
    /// frame in the flow or a mutator it calls cannot break one). Three tiers:
    /// - immutable core: every field fixed when the event was created
    ///   (`header`, `created_slot`, `organizer`, `capacity`, `price_lamports`,
    ///   `name`, `bump`, `_reserved`).
    /// - monotonic counter: the sales total never decreases.
    /// - legal status step: `prev.status -> self.status` is an
    ///   [`is_legal_event_status_transition`] edge.
    #[cfg(creusot)]
    #[logic(open(crate), inline)]
    fn is_legal_transition_from(self, prev: Self) -> bool {
        pearlite! {
            self.header == prev.header
                && self.created_slot == prev.created_slot
                && self.organizer == prev.organizer
                && self.capacity == prev.capacity
                && self.price_lamports == prev.price_lamports
                && self.name == prev.name
                && self.bump == prev.bump
                && self._reserved == prev._reserved
                && self.sold@ >= prev.sold@
                && is_legal_event_status_transition(prev.status, self.status)
        }
    }

    /// Builds a fresh sales-open event with a zeroed sales counter — the base
    /// case for `EventOps::invariant`. Expects a non-default organizer and
    /// bounded economics, established by the creating instruction flow.
    #[cfg_attr(creusot, requires(organizer.is_set()
        && capacity.int() * price_lamports.int() <= u64::MAX@))]
    #[cfg_attr(
        creusot,
        ensures(result.is_initial(organizer, capacity, price_lamports, name, bump, created_slot))
    )]
    fn new(
        organizer: Address,
        capacity: NonZeroU64,
        price_lamports: NonZeroU64,
        name: EventName,
        bump: u8,
        created_slot: u64,
    ) -> Self
    where
        Self: Sized,
    {
        Event {
            header: account_header(),
            created_slot,
            updated_slot: created_slot,
            organizer,
            capacity,
            price_lamports,
            sold: 0,
            name,
            bump,
            status: EventStatus::SalesOpen,
            _reserved: zero_reserved(),
        }
    }

    /// Returns Ok when the supplied authority is this event's organizer.
    #[cfg_attr(creusot, ensures(match result {
        Ok(_) => (*self).organizer == *organizer,
        Err(_) => (*self).organizer != *organizer,
    }))]
    fn validate_organizer(&self, organizer: &Address) -> ProgramResult {
        if !self.organizer.ax_eq(organizer) {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    /// Returns Ok when this event's ticket sales are open.
    #[cfg_attr(creusot, ensures(match result {
        Ok(_) => (*self).is_status_sales_open(),
        Err(_) => !(*self).is_status_sales_open(),
    }))]
    #[allow(
        clippy::match_wildcard_for_single_variants,
        reason = "the wildcard rejects every non-open status, staying correct as `EventStatus` gains variants"
    )]
    fn require_sales_open(&self) -> ProgramResult {
        match self.status {
            EventStatus::SalesOpen => Ok(()),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    /// Records a ticket sale, clamping `requested` to remaining capacity and
    /// returning the quantity actually sold.
    #[cfg_attr(creusot, requires((*self).invariant()))]
    #[cfg_attr(creusot, ensures((^self).invariant()))]
    #[cfg_attr(creusot, ensures(match result {
        Ok(actual) => (*self).is_status_sales_open()
            && actual.int() == int_min(
                requested.int(),
                (*self).capacity.int() - (*self).sold@,
            )
            && actual.int() * (*self).price_lamports.int() <= u64::MAX@
            && (^self).sold@ == (*self).sold@ + actual.int(),
        Err(_) => !(*self).is_status_sales_open()
            || (*self).sold@ == (*self).capacity.int(),
    }))]
    #[cfg_attr(creusot, ensures(match result {
        Ok(_) => (^self) == Event { sold: (^self).sold, ..*self },
        Err(_) => (^self) == *self,
    }))]
    fn record_sale(&mut self, requested: NonZeroU64) -> Result<NonZeroU64, ProgramError> {
        self.require_sales_open()?;

        let remaining = self
            .capacity
            .ax_get()
            .checked_sub(self.sold)
            .and_then(NonZeroU64::ax_new)
            .ok_or(ProgramError::InvalidAccountData)?;

        let actual = remaining.ax_min(requested);
        self.sold =
            self.sold.checked_add(actual.ax_get()).ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(actual)
    }

    /// Computes the lamport payment due for `quantity` tickets at this event's
    /// unit price.
    #[cfg_attr(creusot, ensures(match result {
        Ok(payment) => payment.int() == quantity.int() * (*self).price_lamports.int(),
        Err(_) => quantity.int() * (*self).price_lamports.int() > u64::MAX@,
    }))]
    fn payment_for(&self, quantity: NonZeroU64) -> Result<NonZeroU64, ProgramError> {
        quantity.ax_checked_mul(self.price_lamports).ok_or(ProgramError::ArithmeticOverflow)
    }

    /// Closes ticket sales.
    #[cfg_attr(creusot, requires((*self).invariant()))]
    #[cfg_attr(creusot, ensures((^self).invariant()))]
    #[cfg_attr(creusot, ensures(match result {
        Ok(_) => (*self).is_status_sales_open() && (^self).is_status_sales_closed(),
        Err(_) => !(*self).is_status_sales_open(),
    }))]
    #[cfg_attr(creusot, ensures(match result {
        Ok(_) => (^self) == Event { status: (^self).status, ..*self },
        Err(_) => (^self) == *self,
    }))]
    fn close_sales(&mut self) -> ProgramResult {
        self.require_sales_open()?;
        self.status = EventStatus::SalesClosed;
        Ok(())
    }
}
