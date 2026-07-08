//! Shared wire-format primitives used by the program and its clients.

mod account_header;
mod bounded_string;
mod reserved;

pub use account_header::{ACCOUNT_HEADER_BYTES, AccountHeader, AccountHeaderSpec};
pub use bounded_string::{BoundedString, BoundedStringError};
pub use reserved::Reserved;
pub use solana_address::Address;

/// Number of bytes in an account or instruction discriminator.
pub const DISCRIMINATOR_BYTES: usize = 7;

/// Fixed-width account or instruction discriminator bytes.
pub type Discriminator = [u8; DISCRIMINATOR_BYTES];

/// Returns whether every byte in `bytes` is zero, scanning eight bytes at a
/// time through `u64` words for the fixed-capacity padding checks.
pub(crate) fn all_zero(bytes: &[u8]) -> bool {
    // SAFETY: every bit pattern is valid for `u64`, and `align_to` only exposes
    // the portion of `bytes` that is correctly aligned and sized for `u64`.
    let (head, words, tail) = unsafe { bytes.align_to::<u64>() };

    head.iter().all(|&byte| byte == 0)
        && words.iter().all(|&word| word == 0)
        && tail.iter().all(|&byte| byte == 0)
}
