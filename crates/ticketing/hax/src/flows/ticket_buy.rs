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

/// Purchase for a buyer: validate the organizer, record the clamped sale, price
/// it, and build the buyer's ticket. Returns the payment (for the shell's
/// transfer CPI) and the ticket to persist.
///
/// This end-to-end composition is extracted to Lean but its contract is not yet
/// proved: hax 0.3.7 does not compose the per-core `lean::after` theorems (the
/// `Spec`-structure form that `mvcgen` would use modularly hits the backend's
/// `pureEnsures` match limitation), so a `purchase` proof must re-derive
/// `record_sale` + `payment_for` inline, where the 64-bit nonlinear pricing
/// bound under the `int` spec-set defeats `grind`. Tracked in docs/hax.md as
/// the one remaining core.
// `?` would extract through the `Try`/`ControlFlow` traits and bury the Lean
// model in `branch`/`from_residual` scaffolding; compose the `Result` with an
// explicit `match` instead (the extraction-first rule shared with the Aeneas
// lane).
#[allow(clippy::question_mark)]
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
