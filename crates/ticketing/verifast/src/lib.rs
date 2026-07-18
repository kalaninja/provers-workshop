//! VeriFast separation-logic lane scaffold.
//!
//! This first commit proves one deliberately small, non-vacuous function and
//! establishes the hermetic Rust -> VeriFast pipeline: `bin/verifast` drives
//! `rustc +nightly-2025-11-25` to MIR, then the built-in Redux prover
//! discharges the contract. The ticketing program and its real decision cores
//! arrive only after that pipeline is known to work end to end. VeriFast reads
//! the contract from the `//@` comments below, which are invisible to `rustc`,
//! so there is no `cfg(verifast)` and no verifier dependency - ordinary builds
//! see plain Rust, like the Aeneas lane.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

/// Returns a fixed answer, pinned exactly by the VeriFast postcondition.
///
/// The postcondition `result == 42` is non-vacuous: a broken toolchain or prover would fail it
/// rather than pass silently. `#[rustfmt::skip]` stops rustfmt from moving the `//@` contract, which
/// VeriFast requires to sit between the signature and the body.
#[rustfmt::skip]
#[must_use]
pub fn smoke() -> i32
//@ req true;
//@ ens result == 42;
{
    42
}

#[cfg(test)]
mod tests {
    use super::smoke;

    #[test]
    fn smoke_returns_the_verified_answer() {
        assert_eq!(smoke(), 42);
    }
}
