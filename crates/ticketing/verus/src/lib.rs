//! Verus lane for the ticketing program.
//!
//! Mirrors `ticketing-creusot`/`ticketing-kani`: the verified cores will live
//! here with the Verus spec surface gated behind the `verus` Cargo feature.
//! This is the skeleton - one smoke proof that proves the verus-tooling image
//! works end to end.
//!
//! Everything Verus-specific is behind `#[cfg(feature = "verus")]`, so ordinary
//! `cargo check --workspace` and SBF builds compile this crate as an empty lib.
//! Unlike cargo-kani/cargo-creusot (which set `--cfg kani`/`--cfg creusot`),
//! cargo-verus exposes no cargo-visible cfg, so the lane gates by feature;
//! `cargo verus verify --features verus` turns the surface on and the driver
//! injects the precompiled vstd.

#![warn(missing_docs)]

#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg(feature = "verus")]
verus! {
/// Smoke proof: a trivial requires/ensures discharged by Verus's bundled Z3, so a green
/// `just verus verify` confirms the toolchain + verus-driver + vstd + Z3 are wired end to end.
pub fn smoke_triple(x: u32) -> (r: u32)
    requires x <= 1_000,
    ensures r == 3 * x,
{
    3 * x
}
}
