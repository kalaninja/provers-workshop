//! Event accounting/lifecycle logic - the pure decisions the flow cores call.
//!
//! Charon extracts these functions and Aeneas proves their contracts in Lean.
//! State is read and written only through [`crate::domain::glue`] (opaque), so
//! all the reasoning here - the clamp, the no-underflow/no-overflow arithmetic,
//! no overselling, the status step - runs on `u64` and enums that Aeneas models
//! precisely. Unlike the Flux lane, the nonlinear `capacity * price` bound and
//! the pricing product are *proved*, not trusted.
//!
//! Control flow is deliberately `?`-free: each `?` extracts through the
//! `Try`/`ControlFlow` trait machinery and buries the Lean model under
//! `branch`/`from_residual` scaffolding, so guards branch on the boolean glue
//! accessors directly and `Result`s compose by explicit `match`.
use core::num::NonZeroU64;

use pinocchio::error::ProgramError;
use ticketing_interface::Event;

use crate::domain::glue::{
    get_capacity, get_price, get_sold, is_open, nz_get, set_sold, set_status_closed,
};

/// Records a ticket sale, clamping `requested` to remaining capacity and
/// returning the quantity actually sold (`min(requested, capacity - sold)`,
/// positive). Only `sold` changes and it grows by exactly the returned
/// quantity, never past capacity; the other fields are pinned. The
/// `capacity - sold` and `sold + actual` steps are proved free of
/// under/overflow from the event invariant (`sold <= capacity <= u64::MAX`).
pub fn record_sale(event: &mut Event, requested: NonZeroU64) -> Result<u64, ProgramError> {
    if !is_open(event) {
        return Err(ProgramError::InvalidAccountData);
    }

    let cap = get_capacity(event);
    let sold = get_sold(event);

    let remaining = cap - sold;
    if remaining == 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    let requested = nz_get(requested);
    let actual = if remaining <= requested { remaining } else { requested };

    set_sold(event, sold + actual);
    Ok(actual)
}

/// Computes the lamport payment due for `quantity` tickets at this event's unit
/// price. The clamp guarantees `quantity <= capacity`, so the product neither
/// overflows (`capacity * price <= u64::MAX` is the event invariant) nor is
/// zero (both factors positive) - both proved in Lean.
pub fn payment_for(e: &Event, quantity: u64) -> u64 {
    let price = get_price(e);
    quantity * price
}

/// Closes ticket sales. Only `status` changes; `open -> closed`.
pub fn close_sales(event: &mut Event) -> Result<(), ProgramError> {
    if !is_open(event) {
        return Err(ProgramError::InvalidAccountData);
    }
    set_status_closed(event);
    Ok(())
}
