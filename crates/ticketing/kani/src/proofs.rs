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
//!
//! Because the checked domain is deliberately narrowed and every `requires` is
//! assumed, each harness ends with `kani::cover!` checks that the interesting
//! outcomes (both result arms, the sale clamp) stay reachable — a cover that
//! becomes unsatisfiable fails the run, so over-constraining the input space
//! cannot silently turn the `ensures` vacuous.

use core::num::{NonZeroU8, NonZeroU64};

use bytemuck::Contiguous;
use pinocchio::Address;
use ticketing_interface::{
    Event, EventName, EventStatus,
    wire::{AccountHeader, Reserved},
};

use crate::flows::{event_close::close_event, event_create::create_event, ticket_buy::purchase};

// ---- arbitrary builders
// ------------------------------------------------------

/// An arbitrary non-zero capacity, bounded to `u8` by drawing from the typed
/// `NonZeroU8` domain (see the module docs). Every symbolic-by-symbolic product
/// in the cores has `capacity` (or a quantity clamped by it) as one operand, so
/// this bound is what keeps CBMC tractable while leaving `price` full-width and
/// the overflow boundary reachable. Full-width non-zero values (price,
/// requested quantity) come straight from `kani::any::<NonZeroU64>()` at the
/// call sites.
fn any_capacity() -> NonZeroU64 {
    NonZeroU64::from(kani::any::<NonZeroU8>())
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

/// An arbitrary event lifecycle status, drawn from the full `Contiguous`
/// discriminant range — a status variant added later is picked up here
/// automatically instead of silently narrowing the checked domain.
fn any_status() -> EventStatus {
    let raw = kani::any_where(|raw: &u8| EventStatus::from_integer(*raw).is_some());
    EventStatus::from_integer(raw).expect("assumed a valid discriminant")
}

/// An arbitrary event. `header` and `_reserved` are single-inhabitant types
/// (only the canonical header bytes and all-zero padding are constructible),
/// and the name is fixed (see `a_name`); every other field is symbolic.
/// `purchase`/`close_event`'s `requires(event.invariant())` constrains it.
fn any_event() -> Event {
    Event {
        header: AccountHeader::new(),
        created_slot: kani::any(),
        updated_slot: kani::any(),
        organizer: any_address(),
        capacity: any_capacity(),
        price_lamports: kani::any(),
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
    let result = create_event(
        any_address(),
        any_capacity(),
        kani::any(),
        a_name(),
        kani::any(),
        kani::any(),
    );

    kani::cover!(result.is_ok(), "creating an event can succeed");
    kani::cover!(result.is_err(), "the economics overflow rejection is reachable");
}

/// Checks `purchase`'s contract over an arbitrary invariant-satisfying event.
#[kani::proof_for_contract(purchase)]
#[kani::solver(kissat)]
fn purchase_contract() {
    let mut event = any_event();
    let organizer = any_address();
    let event_addr = any_address();
    let buyer = any_address();
    let requested: NonZeroU64 = kani::any();
    let result =
        purchase(&mut event, &organizer, &event_addr, &buyer, requested, kani::any(), kani::any());

    kani::cover!(result.is_ok(), "a purchase can succeed");
    kani::cover!(result.is_err(), "a purchase can be rejected");
    kani::cover!(
        matches!(&result, Ok((_, ticket)) if ticket.quantity < requested),
        "the sale clamp is reachable"
    );
}

/// Checks `close_event`'s contract over an arbitrary invariant-satisfying
/// event.
#[kani::proof_for_contract(close_event)]
#[kani::solver(kissat)]
fn close_event_contract() {
    let mut event = any_event();
    let organizer = any_address();
    let result = close_event(&mut event, &organizer);

    kani::cover!(result.is_ok(), "closing sales can succeed");
    kani::cover!(result.is_err(), "closing sales can be rejected");
}
