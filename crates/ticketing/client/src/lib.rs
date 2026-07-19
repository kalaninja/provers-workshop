//! Codama-generated Rust client for the ticketing program.
//!
//! The [`generated`] module is rendered from `idl/ticketing.json` by
//! `just solana gen-client` and must not be edited by hand. The crate root
//! re-exports the generated API and lifts `TICKETING_ID` and `shared` to
//! where the generated code refers to them (`crate::...`).
//!
//! # Building an instruction
//!
//! The generated account types provide the canonical PDA helpers. Instruction
//! arguments mirror the IDL exactly, including explicit reserved bytes:
//!
//! ```
//! use solana_address::Address;
//! use ticketing_client::{
//!     TICKETING_ID,
//!     accounts::Event,
//!     instructions::{EventCreate, EventCreateInstructionArgs},
//!     types::EventName,
//! };
//!
//! let organizer = Address::from([1; 32]);
//! let (event, _bump) = Event::find_pda(&organizer);
//! let label = b"rust-summit";
//! let mut name_bytes = [0; 64];
//! name_bytes[..label.len()].copy_from_slice(label);
//!
//! let instruction = EventCreate {
//!     organizer,
//!     event,
//!     // The system program address is 32 zero bytes (`111111...` in base58).
//!     system_program: Address::from([0; 32]),
//! }
//! .instruction(EventCreateInstructionArgs {
//!     capacity: 500,
//!     price_lamports: 2_000_000,
//!     name: EventName { len: label.len() as u16, bytes: name_bytes },
//!     // Required by the generated API and required to be all-zero on-chain.
//!     reserved: [0; 6],
//! });
//!
//! assert_eq!(instruction.program_id, TICKETING_ID);
//! ```
//!
//! `EventName` stores bytes, not validated text. Validate UTF-8 in the client
//! before displaying it when the application requires a textual name.
//!
//! # Known codegen caveats
//!
//! **Account decoding.** The generated `Account::from_bytes` /
//! `TryFrom<&AccountInfo>` decoders borsh-deserialize from offset 0 and do
//! **not** validate the 8-byte header against the account's
//! `*_DISCRIMINATOR`, nor (in the `fetch` helpers) that the account is owned
//! by [`TICKETING_ID`]. Bytes with a compatible layout can therefore decode
//! even when they belong to the wrong account type or program. Callers that
//! decode untrusted addresses should use [`decode::decode_checked`], or
//! `decode::decode_checked_account` with the `fetch` feature, to gate on the
//! discriminator (and owner) first.
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

#[cfg(feature = "fetch")]
pub use decode::decode_checked_account;
pub use decode::{CheckedAccount, DecodeError, decode_checked};
pub use generated::{
    accounts, errors, instructions, programs, programs::TICKETING_ID, shared, types,
};
