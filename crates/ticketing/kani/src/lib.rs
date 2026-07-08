//! Kani bounded-model-checking lane for the ticketing program.
//!
//! Mirrors `ticketing-creusot`: the verified cores will live here with a `cfg(kani)` spec
//! layer (domain predicates as ordinary `#[cfg(kani)]` boolean fns, contracts as
//! `#[cfg_attr(kani, kani::requires/ensures)]`). This is the skeleton — one smoke harness
//! that proves the kani-tooling image works end to end.
//!
//! Everything Kani-specific is behind `#[cfg(kani)]`, so ordinary `cargo check --workspace`
//! and SBF builds compile this crate as an empty lib. `cargo kani` sets `--cfg kani` and
//! injects the `kani` crate itself — no Cargo dependency on `kani` is required.

#![warn(missing_docs)]

// Smoke harness: exercises kani::any / kani::assume / assert and CBMC arithmetic so a green
// run confirms the compiler + CBMC backend are wired. Loop-free and unwind-bound-free.
#[cfg(kani)]
#[kani::proof]
fn smoke_bounded_add_no_overflow() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    kani::assume(a <= 1_000);
    kani::assume(b <= 1_000);
    let sum = a + b; // CBMC also proves this cannot overflow given the assumptions
    assert!(sum <= 2_000);
}
