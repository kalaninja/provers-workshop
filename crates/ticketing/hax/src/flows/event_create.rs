//! Verified decision core for the `event_create` instruction.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, EventName};

use crate::domain::glue::{event_new, nz_get};

/// Creates an event, rejecting economics whose worst-case payment (`capacity *
/// price_lamports`) would overflow `u64` - the bound the event invariant then
/// carries for the account's whole life. On success the event is initial
/// (zeroed counter, pinned slots/organizer/bump, sales open) and satisfies the
/// invariant.
///
/// Proved in Lean: `Ok` yields an initial event satisfying the invariant, and
/// the error occurs exactly when `capacity * price_lamports > u64::MAX`.
#[cfg_attr(hax, hax_lib::lean::after(
    "
set_option hax_mvcgen.specset \"int\" in
theorem create_event_spec
    (organizer : solana_address.Address)
    (capacity price_lamports : core_models.num.nonzero.NonZero u64)
    (name : ticketing_interface.wire.bounded_string.BoundedString (64 : usize))
    (bump : u8) (created_slot : u64) :
  ⦃ ⌜ solana_address.is_set organizer ⌝ ⦄
  ticketing_hax.flows.event_create.create_event organizer capacity price_lamports name bump created_slot
  ⦃ ⇓ res => ⌜ match res with
    | .Ok ev =>
      EventInv ev ∧ ev.organizer = organizer ∧
      ev.capacity = core_models.num.nonzero.nzVal capacity ∧
      ev.price = core_models.num.nonzero.nzVal price_lamports ∧
      ev.sold = 0 ∧ ev.closed = false ∧ ev.name = name ∧
      ev.createdSlot = created_slot ∧ ev.updatedSlot = created_slot ∧ ev.bump = bump
    | .Err e =>
      e = solana_program_error.ProgramError.ArithmeticOverflow ∧
      18446744073709551615 <
        (core_models.num.nonzero.nzVal capacity).toNat
          * (core_models.num.nonzero.nzVal price_lamports).toNat ⌝ ⦄
:= by
  hax_mvcgen [ticketing_hax.flows.event_create.create_event]
  case vc1.h =>
    intro hc
    have := core_models.num.nonzero.nzVal_pos price_lamports
    rw [hc] at this; simp at this
  case vc2.success.isTrue =>
    rename_i hr hcond
    have hMAX : core_models.num.Impl_9.MAX.toNat = 18446744073709551615 := by decide
    have hp0 := core_models.num.nonzero.nzVal_pos price_lamports
    rw [hMAX] at hr
    simp only [decide_eq_true_eq, gt_iff_lt, UInt64.lt_iff_toNat_lt] at hcond
    exact ⟨trivial,
      @div_lt_mul_gt (core_models.num.nonzero.nzVal capacity).toNat
        (core_models.num.nonzero.nzVal price_lamports).toNat hp0 (by omega)⟩
  case vc3.success.isFalse =>
    rename_i hr hcond
    have hMAX : core_models.num.Impl_9.MAX.toNat = 18446744073709551615 := by decide
    have hc0 := core_models.num.nonzero.nzVal_pos capacity
    have hp0 := core_models.num.nonzero.nzVal_pos price_lamports
    rw [hMAX] at hr
    simp only [Bool.not_eq_true, decide_eq_false_iff_not, gt_iff_lt,
      UInt64.lt_iff_toNat_lt, Nat.not_lt] at hcond
    rw [hr] at hcond
    simp only [EventInv]
    exact ⟨‹solana_address.is_set organizer›, hc0, hp0, Nat.zero_le _,
      @le_div_mul_le (core_models.num.nonzero.nzVal capacity).toNat
        (core_models.num.nonzero.nzVal price_lamports).toNat hcond⟩
"
))]
pub fn create_event(
    organizer: Address,
    capacity: NonZeroU64,
    price_lamports: NonZeroU64,
    name: EventName,
    bump: u8,
    created_slot: u64,
) -> Result<Event, ProgramError> {
    let c = nz_get(capacity);
    let p = nz_get(price_lamports);
    // Overflow gate expressed with division rather than `checked_mul`: hax's Lean
    // prelude models `/` and `>` but not `u64::checked_mul`, so this keeps the
    // bound *proved* (not trusted) while staying behavior-identical - `p >= 1`
    // always (it came from a `NonZeroU64`), so the division cannot fault (the
    // Lean side proves this from `nzVal_pos`) and `c > u64::MAX / p` holds
    // exactly when `c * p > u64::MAX`, i.e. when `c.checked_mul(p)` is `None`.
    // Recorded as a hax code-bending note in docs/hax.md.
    if c > u64::MAX / p {
        Err(ProgramError::ArithmeticOverflow)
    } else {
        Ok(event_new(organizer, c, p, name, bump, created_slot))
    }
}
