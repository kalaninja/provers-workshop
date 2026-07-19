//! Verified decision core for the `ticket_buy` instruction.
//!
//! Holds the pure purchase decision - validate the organizer, clamp the sale,
//! price it, and build the buyer's ticket - so every constructor/mutator call
//! runs at a verified call site. Proved in Lean end-to-end: the event invariant
//! is preserved, immutable fields are fixed, `sold` grows by exactly the
//! ticket's clamped quantity, the payment is `quantity * price` without
//! overflow, the ticket pins the buyer/event/payment, and on error the event is
//! unchanged.
use core::num::NonZeroU64;

use pinocchio::{Address, error::ProgramError};
use ticketing_interface::{Event, Ticket};

use crate::domain::{
    event::{payment_for, record_sale},
    glue::{nz_from, organizer_eq, ticket_new},
};

/// Purchase for a buyer: validate the organizer, record the clamped sale on the
/// event, price it, and build the buyer's ticket. Returns the payment (for the
/// shell's transfer CPI) and the freshly-built ticket (for the shell to
/// persist).
///
/// The contract is the embedded Lean Hoare triple `purchase_spec` below: from
/// the event invariant and set event/buyer addresses, the invariant is
/// preserved, only `sold` moves (every other field pinned) and it grows by
/// exactly the ticket's clamped quantity (positive, bounded by the request and
/// the remaining capacity), the ticket carries its invariant and pins the
/// buyer/event/payment/bump/slots, the payment is `quantity * price` (positive,
/// no overflow), and on error the event is unchanged. Mirrors the Creusot
/// lane's `purchase` coverage.
///
/// Unlike the leaf cores, this end-to-end composition reuses the already-proved
/// `record_sale_spec` and `payment_for_spec` as `mvcgen` step specs -
/// `attribute [spec]` registers them and the `-record_sale`/`-payment_for`
/// erase suppresses the backend's default `@[spec]` unfold - so `record_sale`'s
/// clamp and `payment_for`'s nonlinear no-overflow stay discharged inside their
/// own proofs and never re-enter this one. That is the modular composition hax
/// `0.3.7` does not wire up on its own (the `requires`/`ensures` -> `Spec` path
/// is blocked by the `pureEnsures` `Result` match); registering the
/// hand-written triples as specs is the way through.
// `?` would extract through the `Try`/`ControlFlow` traits and bury the Lean
// model in `branch`/`from_residual` scaffolding; compose the `Result` with an
// explicit `match` instead (the extraction-first rule shared with the Aeneas
// lane).
#[allow(clippy::question_mark)]
#[cfg_attr(
    hax,
    hax_lib::lean::after(
        "
attribute [spec] ticketing_hax.domain.event.record_sale_spec
attribute [spec] ticketing_hax.domain.event.payment_for_spec

set_option maxHeartbeats 1000000 in
theorem purchase_spec
    (event : ticketing_interface.state.event.Event)
    (organizer : solana_address.Address)
    (event_addr : solana_address.Address)
    (buyer : solana_address.Address)
    (requested : core_models.num.nonzero.NonZero u64)
    (bump : u8) (created_slot : u64) :
  ⦃ ⌜ EventInv event ∧ solana_address.is_set event_addr ∧ solana_address.is_set buyer ⌝ ⦄
  ticketing_hax.flows.ticket_buy.purchase event organizer event_addr buyer requested bump created_slot
  ⦃ ⇓ ⟨ev', res⟩ => ⌜
    EventInv ev' ∧ EventLegalTransition ev' event ∧
    ev'.capacity = event.capacity ∧ ev'.price = event.price ∧
    ev'.organizer = event.organizer ∧ ev'.closed = event.closed ∧
    ev'.name = event.name ∧ ev'.createdSlot = event.createdSlot ∧
    ev'.updatedSlot = event.updatedSlot ∧ ev'.bump = event.bump ∧
    (match res with
     | .Ok ⟨payment, ticket⟩ =>
       event.organizer = organizer ∧ event.closed = false ∧ TicketInv ticket ∧
       ticket.event = event_addr ∧ ticket.buyer = buyer ∧ ticket.paid = payment ∧
       ticket.bump = bump ∧ ticket.createdSlot = created_slot ∧
       ticket.updatedSlot = created_slot ∧
       0 < (core_models.num.nonzero.nzVal ticket.quantity).toNat ∧
       (core_models.num.nonzero.nzVal ticket.quantity).toNat
         ≤ (core_models.num.nonzero.nzVal requested).toNat ∧
       (core_models.num.nonzero.nzVal ticket.quantity).toNat
         ≤ event.capacity.toNat - event.sold.toNat ∧
       ev'.sold.toNat
         = event.sold.toNat + (core_models.num.nonzero.nzVal ticket.quantity).toNat ∧
       (core_models.num.nonzero.nzVal payment).toNat
         = (core_models.num.nonzero.nzVal ticket.quantity).toNat * event.price.toNat
     | .Err _ => ev' = event) ⌝ ⦄
:= by
  hax_mvcgen [ticketing_hax.flows.ticket_buy.purchase,
      -ticketing_hax.domain.event.record_sale,
      -ticketing_hax.domain.event.payment_for] <;>
    grind [EventInv, EventLegalTransition, TicketInv,
      core_models.num.nonzero.nzVal_pos, core_models.num.nonzero.nzVal_nzMk,
      core_models.num.nonzero.nzVal_nzMk_of_pos,
      toNat_pos_of_ne, UInt64.lt_iff_toNat_lt, UInt64.le_iff_toNat_le, UInt64.toNat_lt]
"
    )
)]
pub fn purchase(
    event: &mut Event,
    organizer: &Address,
    event_addr: &Address,
    buyer: &Address,
    requested: NonZeroU64,
    bump: u8,
    created_slot: u64,
) -> Result<(NonZeroU64, Ticket), ProgramError> {
    if !organizer_eq(event, organizer) {
        return Err(ProgramError::InvalidAccountData);
    }
    let actual = match record_sale(event, requested) {
        Ok(actual) => actual,
        Err(e) => return Err(e),
    };
    let payment = nz_from(payment_for(event, actual));
    let ticket = ticket_new(*event_addr, *buyer, payment, nz_from(actual), bump, created_slot);
    Ok((payment, ticket))
}
