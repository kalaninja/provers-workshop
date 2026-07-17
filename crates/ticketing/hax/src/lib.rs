//! hax-to-Lean lane scaffold.
//!
//! This first commit proves one deliberately small, non-vacuous function and
//! establishes the hermetic Rust -> hax -> Lean pipeline. The ticketing program
//! and its real decision cores are added only after that pipeline is known to
//! work end to end.
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

/// Squares a small byte without overflowing it.
///
/// The hax precondition is the exact overflow boundary needed by the Lean
/// proof: widening it from `x < 16` to `x < 17` admits `16`, whose square wraps
/// to zero and makes the postcondition false.
#[cfg_attr(hax, hax_lib::requires(x < 16))]
#[cfg_attr(hax, hax_lib::ensures(|result| result >= x))]
#[cfg_attr(hax, hax_lib::lean::proof("by unfold smoke_square; hax_bv_decide"))]
#[must_use]
pub fn smoke_square(x: u8) -> u8 {
    x * x
}

#[cfg(test)]
mod tests {
    use super::smoke_square;

    #[test]
    fn squares_inside_the_verified_domain() {
        assert_eq!(smoke_square(0), 0);
        assert_eq!(smoke_square(15), 225);
    }
}
