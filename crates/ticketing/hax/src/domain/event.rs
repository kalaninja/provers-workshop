//! Event accounting/lifecycle logic - the pure decisions the flow cores call.
//!
//! hax extracts these functions and their inline contracts are proved in Lean.
//! State is read and written only through [`crate::domain::glue`] (opaque), so
//! all the reasoning here - the clamp, the no-underflow/no-overflow arithmetic,
//! no overselling, the status step - runs on `u64` and enums that Lean models
//! precisely. Like the Aeneas lane and unlike Flux, the nonlinear `capacity *
//! price` bound and the pricing product are *proved*, not trusted.
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
#[cfg_attr(
    hax,
    hax_lib::lean::after(
        "
set_option maxHeartbeats 1000000 in
set_option hax_mvcgen.specset \"int\" in
theorem record_sale_spec (event : ticketing_interface.state.event.Event)
    (requested : core_models.num.nonzero.NonZero u64) :
  ⦃ ⌜ EventInv event ⌝ ⦄
  ticketing_hax.domain.event.record_sale event requested
  ⦃ ⇓ ⟨ev', res⟩ => ⌜
    EventInv ev' ∧ EventLegalTransition ev' event ∧
    ev'.capacity = event.capacity ∧ ev'.price = event.price ∧
    ev'.organizer = event.organizer ∧ ev'.closed = event.closed ∧
    ev'.name = event.name ∧ ev'.createdSlot = event.createdSlot ∧
    ev'.updatedSlot = event.updatedSlot ∧ ev'.bump = event.bump ∧
    (match res with
     | .Ok actual =>
       event.closed = false ∧ 0 < actual.toNat ∧
       actual.toNat ≤ (core_models.num.nonzero.nzVal requested).toNat ∧
       actual.toNat ≤ event.capacity.toNat - event.sold.toNat ∧
       ev'.sold.toNat = event.sold.toNat + actual.toNat ∧
       actual.toNat ≤ event.capacity.toNat ∧
       actual.toNat * event.price.toNat ≤ 18446744073709551615
     | .Err _ =>
       ev' = event ∧ (event.closed = true ∨ event.sold.toNat = event.capacity.toNat)) ⌝ ⦄
:= by
  have hnz := core_models.num.nonzero.nzVal_pos requested
  have hcap64 := UInt64.toNat_lt event.capacity
  have hsold64 := UInt64.toNat_lt event.sold
  hax_mvcgen [ticketing_hax.domain.event.record_sale] <;>
    first
      | grind [EventInv, EventLegalTransition, mul_le_max, core_models.num.nonzero.nzVal_pos,
          UInt64.lt_iff_toNat_lt, UInt64.le_iff_toNat_le, UInt64.toNat_lt]
      | (simp_all only [EventInv, EventLegalTransition, UInt64.le_iff_toNat_le,
            decide_eq_true_eq, Bool.not_eq_true, Bool.not_not, beq_eq_false_iff_ne, ne_eq]
         repeat' first
           | rfl
           | assumption
           | exact mul_le_max (cap := event.capacity.toNat) (by omega) (by omega)
           | exact toNat_pos_of_ne (by assumption)
           | omega
           | constructor)
"
    )
)]
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
#[cfg_attr(
    hax,
    hax_lib::lean::after(
        "
set_option hax_mvcgen.specset \"int\" in
theorem payment_for_spec (e : ticketing_interface.state.event.Event) (quantity : u64) :
  ⦃ ⌜ EventInv e ∧ quantity.toNat ≤ e.capacity.toNat ∧ 0 < quantity.toNat ⌝ ⦄
  ticketing_hax.domain.event.payment_for e quantity
  ⦃ ⇓ p => ⌜ p.toNat = quantity.toNat * e.price.toNat ∧ 0 < p.toNat ⌝ ⦄
:= by
  hax_mvcgen [ticketing_hax.domain.event.payment_for]
  · intro hmo
    obtain ⟨hinv, hq, _⟩ := ‹EventInv e ∧ _ ∧ _›
    simp only [EventInv] at hinv
    obtain ⟨_, _, _, _, hbound⟩ := hinv
    have h1 := (UInt64.mulOverflow_iff).mp hmo
    have h2 := mul_no_overflow hq hbound
    omega
  · intro hr
    obtain ⟨hinv, _, hqpos⟩ := ‹EventInv e ∧ _ ∧ _›
    simp only [EventInv] at hinv
    obtain ⟨_, _, hprice, _, _⟩ := hinv
    exact ⟨hr, by rw [hr]; exact Nat.mul_pos hqpos hprice⟩
"
    )
)]
pub fn payment_for(e: &Event, quantity: u64) -> u64 {
    let price = get_price(e);
    quantity * price
}

/// Closes ticket sales. Only `status` changes; `open -> closed`.
#[cfg_attr(
    hax,
    hax_lib::lean::after(
        "
theorem close_sales_spec (event : ticketing_interface.state.event.Event) :
  ⦃ ⌜ EventInv event ⌝ ⦄
  ticketing_hax.domain.event.close_sales event
  ⦃ ⇓ ⟨ev', res⟩ => ⌜
    EventInv ev' ∧ EventLegalTransition ev' event ∧
    ev'.capacity = event.capacity ∧ ev'.price = event.price ∧
    ev'.organizer = event.organizer ∧ ev'.sold = event.sold ∧
    ev'.name = event.name ∧ ev'.createdSlot = event.createdSlot ∧
    ev'.updatedSlot = event.updatedSlot ∧ ev'.bump = event.bump ∧
    (match res with
     | .Ok _ => event.closed = false ∧ ev'.closed = true
     | .Err _ => ev' = event ∧ event.closed = true) ⌝ ⦄
:= by
  hax_mvcgen [ticketing_hax.domain.event.close_sales]
    <;> grind [EventInv, EventLegalTransition]
"
    )
)]
pub fn close_sales(event: &mut Event) -> Result<(), ProgramError> {
    if !is_open(event) {
        return Err(ProgramError::InvalidAccountData);
    }
    set_status_closed(event);
    Ok(())
}
