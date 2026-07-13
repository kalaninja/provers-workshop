//! Aeneas lane for the ticketing program.
//!
//! Mirrors `ticketing-creusot`/`ticketing-kani`/`ticketing-verus`/
//! `ticketing-flux`: the verified cores will live here as ordinary Rust. This
//! is the skeleton - one smoke function whose proof, written in Lean under
//! `verif/ticketing_aeneas`, proves the aeneas-tooling image works end to end
//! (Charon extraction -> Aeneas translation -> Lean check).
//!
//! Unlike the other lanes, Aeneas carries **no** verification surface in the
//! Rust source: Charon extracts this crate to an LLBC model, Aeneas translates
//! it to Lean, and the contracts are hand-written Lean that reasons about the
//! generated functions. So there are no spec attributes, no `cfg(aeneas)`, and
//! no verifier dependency - the crate is plain Rust that compiles unchanged on
//! host and SBF targets, and the proofs live entirely outside it.

#![warn(missing_docs)]

/// Smoke function: Charon extracts it, Aeneas translates it to a Lean
/// `smoke_triple`, and the committed Lean proof in `verif/ticketing_aeneas`
/// discharges a real obligation over it (for `x <= 1_000`, the result is
/// `3 * x` with no overflow), so a green `just aeneas verify` confirms the
/// whole toolchain - Charon, Aeneas, and Lean - is wired end to end.
/// The bound makes the proof non-vacuous: widen the body to `4 * x`, or tighten
/// the Lean bound, and the Lean check fails.
#[must_use]
pub fn smoke_triple(x: u32) -> u32 {
    3 * x
}
