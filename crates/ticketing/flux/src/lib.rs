//! Flux lane for the ticketing program.
//!
//! Mirrors `ticketing-creusot`/`ticketing-kani`/`ticketing-verus`: the verified
//! cores will live here re-expressed as Flux refinement types. This is the
//! skeleton - one smoke spec that proves the flux-tooling image works end to
//! end.
//!
//! Flux's attributes strip to no-ops unless `cargo flux` is driving, so
//! ordinary `cargo check --workspace` and SBF builds compile this crate as
//! plain Rust with no gating. Unlike cargo-verus, `cargo flux` sets `--cfg
//! flux`, so verification-only items use `#[cfg(flux)]` / `#[cfg_attr(flux,
//! ...)]` (as under Kani); the `flux_rs` attribute dependency itself stays
//! unconditional because `[target.'cfg(flux)'.dependencies]` does not activate
//! under cargo-flux.

#![warn(missing_docs)]

/// Smoke spec: a refined signature discharged by Flux's liquid-fixpoint/Z3
/// backend, so a green `just flux verify` confirms the whole toolchain -
/// flux-driver, fixpoint, and Z3 - is wired end to end. The output bound forces
/// a real constraint (fixpoint must prove `3 * x <= 3_000` from `x <= 1_000`),
/// so the check exercises the solver and is not vacuous: widen the body to
/// `4 * x`, or tighten the bound, and verification fails.
#[must_use]
#[flux_rs::sig(fn(x: u32{x <= 1_000}) -> u32{v: v <= 3_000})]
pub fn smoke_triple(x: u32) -> u32 {
    3 * x
}
