//! Verified decision core for the `event_close` instruction.
use pinocchio::{Address, error::ProgramError};
use ticketing_interface::Event;

use crate::domain::{event::close_sales, glue::organizer_eq};

/// Closes an event: authorize the organizer, then move `SalesOpen ->
/// SalesClosed`. Immutable fields are fixed and the status step is legal.
///
/// The contract is the embedded Lean Hoare triple `close_event_spec` below
/// (hax's `lean::after`, which attaches a rich Lean postcondition to the
/// extracted function): from the event invariant, the invariant is
/// preserved, every field but `status` is pinned, and `Ok` holds exactly when
/// the organizer matches and sales were open (then `open -> closed`), while on
/// error the event is unchanged. Mirrors the Creusot lane's `close_event`
/// coverage; proved by `hax_mvcgen` + `grind` over the glue models.
#[cfg_attr(
    hax,
    hax_lib::lean::after(
        "
theorem close_event_spec
    (event : ticketing_interface.state.event.Event)
    (organizer : solana_address.Address) :
  ⦃ ⌜ EventInv event ⌝ ⦄
  ticketing_hax.flows.event_close.close_event event organizer
  ⦃ ⇓ ⟨ev', res⟩ => ⌜
    EventInv ev' ∧
    EventLegalTransition ev' event ∧
    ev'.capacity = event.capacity ∧ ev'.price = event.price ∧
    ev'.organizer = event.organizer ∧ ev'.sold = event.sold ∧
    ev'.name = event.name ∧ ev'.createdSlot = event.createdSlot ∧
    ev'.updatedSlot = event.updatedSlot ∧ ev'.bump = event.bump ∧
    (match res with
     | .Ok _ => event.organizer = organizer ∧ event.closed = false ∧ ev'.closed = true
     | .Err _ => ev' = event ∧ (event.organizer ≠ organizer ∨ event.closed = true)) ⌝ ⦄
:= by
  hax_mvcgen
    [ticketing_hax.flows.event_close.close_event, ticketing_hax.domain.event.close_sales]
    <;> grind [EventInv, EventLegalTransition]
"
    )
)]
pub fn close_event(event: &mut Event, organizer: &Address) -> Result<(), ProgramError> {
    if !organizer_eq(event, organizer) {
        return Err(ProgramError::InvalidAccountData);
    }
    close_sales(event)
}
