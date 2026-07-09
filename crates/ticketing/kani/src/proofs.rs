//! Kani proof harnesses: one `#[kani::proof_for_contract]` per decision core.
//!
//! The contracts live on the functions in `crate::flows` as
//! `#[cfg_attr(kani, kani::requires/ensures)]` (mirroring the Creusot clauses);
//! each harness here supplies arbitrary inputs and calls the function once, and
//! Kani (with `-Z function-contracts`) assumes the `requires` and checks the
//! `ensures`. Kani injects the `kani` crate under `--cfg kani`, so this whole
//! module compiles only under verification.
//!
//! The cores are loop-free, so there are no loop-unwinding bounds — but the
//! economic reasoning is nonlinear and CBMC bit-blasts nonlinear 64-bit
//! arithmetic poorly (proving `purchase`'s error path unreachable needs
//! `quantity * price <= capacity * price <= u64::MAX`). We bound `capacity` to
//! `u8` here while keeping `price` full 64-bit: every symbolic product then has
//! a small (<=255) operand, and full-width `price` keeps the `u64` overflow
//! boundary reachable. The contracts state the real (unbounded) preconditions;
//! only the *checked input domain* is bounded. The harnesses select `kissat`.

use core::num::NonZeroU64;

use pinocchio::Address;
use ticketing_interface::{
    Event, EventName, EventStatus,
    wire::{AccountHeader, Reserved},
};

use crate::flows::{event_close::close_event, event_create::create_event, ticket_buy::purchase};

// ---- arbitrary builders
// ------------------------------------------------------

/// An arbitrary non-zero `u64`, as a `NonZeroU64`. Used for full-width values
/// (price, requested quantity) that are never both operands of a product.
fn any_nonzero() -> NonZeroU64 {
    let v: u64 = kani::any();
    kani::assume(v != 0);
    NonZeroU64::new(v).unwrap()
}

/// An arbitrary non-zero capacity, bounded to `u8` (see the module docs). Every
/// symbolic-by-symbolic product in the cores has `capacity` (or a quantity
/// clamped by it) as one operand, so this bound is what keeps CBMC tractable
/// while leaving `price` full-width and the overflow boundary reachable.
fn any_capacity() -> NonZeroU64 {
    let v: u64 = kani::any();
    kani::assume(v != 0 && v <= u64::from(u8::MAX));
    NonZeroU64::new(v).unwrap()
}

/// An arbitrary address (any 32 bytes).
fn any_address() -> Address {
    Address::from(kani::any::<[u8; 32]>())
}

/// A fixed, valid event name. Name handling is a verbatim byte copy in the
/// cores, so one representative value is enough and keeps the state space
/// small.
fn a_name() -> EventName {
    EventName::from_bytes(b"kani").expect("name fits")
}

/// An arbitrary event lifecycle status.
fn any_status() -> EventStatus {
    if kani::any() { EventStatus::SalesOpen } else { EventStatus::SalesClosed }
}

/// An arbitrary event. Header, name, and reserved padding are fixed (they do
/// not participate in the numeric or status invariants); every other field is
/// symbolic. `purchase`/`close_event`'s `requires(event.invariant())`
/// constrains it.
fn any_event() -> Event {
    Event {
        header: AccountHeader::new(),
        created_slot: kani::any(),
        updated_slot: kani::any(),
        organizer: any_address(),
        capacity: any_capacity(),
        price_lamports: any_nonzero(),
        sold: kani::any(),
        name: a_name(),
        bump: kani::any(),
        status: any_status(),
        _reserved: Reserved::ZERO,
    }
}

// ---- contract harnesses
// ------------------------------------------------------

/// Checks `create_event`'s contract over an arbitrary (bounded-capacity) input.
#[kani::proof_for_contract(create_event)]
#[kani::solver(kissat)]
fn create_event_contract() {
    let _ = create_event(
        any_address(),
        any_capacity(),
        any_nonzero(),
        a_name(),
        kani::any(),
        kani::any(),
    );
}

/// Checks `purchase`'s contract over an arbitrary invariant-satisfying event.
#[kani::proof_for_contract(purchase)]
#[kani::solver(kissat)]
fn purchase_contract() {
    let mut event = any_event();
    let organizer = any_address();
    let event_addr = any_address();
    let buyer = any_address();
    let _ = purchase(
        &mut event,
        &organizer,
        &event_addr,
        &buyer,
        any_nonzero(),
        kani::any(),
        kani::any(),
    );
}

/// Checks `close_event`'s contract over an arbitrary invariant-satisfying
/// event.
#[kani::proof_for_contract(close_event)]
#[kani::solver(kissat)]
fn close_event_contract() {
    let mut event = any_event();
    let organizer = any_address();
    let _ = close_event(&mut event, &organizer);
}
