//! Codama-generated Rust client for the ticketing program.
//!
//! The [`generated`] module is rendered from `idl/ticketing.json` by
//! `just solana gen-client` and must not be edited by hand. The crate root
//! re-exports the generated API and lifts `TICKETING_ID` and `shared` to
//! where the generated code refers to them (`crate::...`).
//!
//! # Known codegen caveats
//!
//! **Account decoding.** The generated `Account::from_bytes` /
//! `TryFrom<&AccountInfo>` decoders borsh-deserialize from offset 0 and do
//! **not** validate the 8-byte header against the account's
//! `*_DISCRIMINATOR`, nor (in the `fetch` helpers) that the account is owned
//! by [`TICKETING_ID`]. Decode only accounts fetched by their canonical PDA; a
//! wrong-type or uninitialized account may decode into a garbage value
//! instead of erroring. Callers that decode untrusted addresses should use
//! [`decode::decode_checked`] / [`decode::decode_checked_account`], which gate
//! on the discriminator (and owner) first.
//!
//! **Remaining accounts (CPI builders).** `@codama/renderers-rust` 3.1.0 emits
//! the generated `*Cpi` / `*CpiBuilder` remaining-account helpers with
//! `is_signer` and `is_writable` transposed (an `(account, is_writable,
//! is_signer)` tuple is read back as `is_signer = .1`, `is_writable = .2`).
//! No ticketing instruction declares remaining accounts, so this is
//! unreachable today; revisit if a future instruction needs them.

// Generated code is vendored output: it leans on `unwrap`/`expect`, carries a
// few unused re-exports, and does not follow the workspace's pedantic lint
// policy, so the whole module opts out.
#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::unwrap_used, unused)]
pub mod generated;

/// Hand-written discriminator/owner-checked account decoders. The generated
/// `from_bytes` validates neither (see the caveats above); these gate on both.
pub mod decode;

pub use decode::{CheckedAccount, DecodeError, decode_checked};
pub use generated::{
    accounts, errors, instructions, programs, programs::TICKETING_ID, shared, types,
};
