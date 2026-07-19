//! Proc-macros for the ticketing program.
//!
//! - `creusot_extension` — a Creusot-aware extension-trait generator that keeps
//!   preconditions synchronized while leaving concrete postconditions on the
//!   generated implementation (module `creusot_extension`).
//! - [`InstructionAccounts`](derive_instruction_accounts) /
//!   [`InstructionArgs`](derive_instruction_args) /
//!   [`StateAccount`](derive_state_account) — wire-layer codegen:
//!   per-instruction typed accounts structs + `parse` (from the
//!   `#[codama(account(...))]` table), the instruction-args payload marker with
//!   a layout-derived `TRAILING_RESERVED`, and the state-account marker impl.
//! - [`CodamaAccount`](derive_codama_account) /
//!   [`CodamaType`](derive_codama_type) /
//!   [`CodamaInstructions`](derive_codama_instructions) /
//!   [`CodamaPda`](derive_codama_pda) — no-op stand-ins (see the note below)
//!   that let the wire crate carry codama's source-level IDL markers without
//!   depending on `codama-macros`.
//!
//! The no-op codama derives emit nothing; they exist only so the wire crate
//! carries the source-level `#[derive(Codama*)]` + `#[codama(...)]` metadata
//! the IDL generator reads straight from the AST — codama matches a derive by
//! its bare name, not by the defining crate — without depending on
//! `codama-macros`, which would drag the whole codama crate graph into every
//! ordinary build. Each declares `attributes(codama)` so the `#[codama(...)]`
//! helper attributes compile; codama (run from the IDL generator) is what
//! actually parses and validates them. The names must stay exactly as codama
//! expects.

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod creusot_extension;
mod instruction_accounts;
mod instruction_args;
mod state_account;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Generates a crate-local extension trait and implementation from one
/// inherent-impl-shaped declaration.
///
/// The macro preserves named parameters and copies `#[cfg_attr(creusot,
/// requires(...))]` to both the generated trait declaration and concrete
/// implementation. Other Creusot method attributes, including `ensures`, stay
/// on the implementation because they may refer to concrete fields or return
/// types unavailable through abstract `Self`.
///
/// # Example
///
/// ```ignore
/// #[creusot_extension(trait EventOps)]
/// pub(crate) impl Event {
///     fn validate(&self) -> ProgramResult {
///         // ...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn creusot_extension(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as creusot_extension::Args);
    let input = parse_macro_input!(input as creusot_extension::ExtensionImpl);
    creusot_extension::expand(&args.trait_name, input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Per-variant typed accounts struct + `parse` constructor, generated from the
/// `#[codama(account(...))]` table on each enum variant.
///
/// Emits one `pub struct <Variant>Accounts<'a>` (one `&'a mut
/// ::pinocchio::AccountView` field per `account(name = "...")`) plus a `parse`
/// that calls `crate::program::accounts::exact_mut::<N>`, then `require_signer`
/// / `require_writable` per flag and a `require_program` / `require_sysvar`
/// identity check for every slot whose `default_value` pins a fixed
/// `program(...)` / `sysvar(...)`. `payer` / `pda(...)` / `account(...)`
/// defaults are resolved by the handler and get no check; an unrecognized
/// fixed-address default is a compile error. The derive reads the same
/// `#[codama(...)]` table the IDL uses, so the runtime account checks and the
/// IDL cannot drift. `optional` slots are rejected — the parser needs an exact
/// account count.
#[proc_macro_derive(InstructionAccounts, attributes(codama))]
pub fn derive_instruction_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    instruction_accounts::expand(&input).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// `InstructionArgs` impl with a layout-derived `TRAILING_RESERVED`.
///
/// If the struct's final field is a `Reserved<N>` pad, sets `TRAILING_RESERVED
/// = N` and emits a compile-time assertion that the pad occupies exactly the
/// last `N` bytes (so the short-payload parser can never zero-fill a real
/// field); otherwise sets `TRAILING_RESERVED = 0`. The value is read from the
/// layout, so it cannot drift from the type.
#[proc_macro_derive(InstructionArgs)]
pub fn derive_instruction_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    instruction_args::expand(&input).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Explicit `StateAccount` marker impl for a concrete account-state struct (one
/// that is `bytemuck::CheckedBitPattern + NoUninit`).
#[proc_macro_derive(StateAccount)]
pub fn derive_state_account(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    state_account::expand(&input).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// No-op stand-in for codama's account marker (see the crate note).
#[proc_macro_derive(CodamaAccount, attributes(codama))]
pub fn derive_codama_account(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// No-op stand-in for codama's defined-type marker (see the crate note).
#[proc_macro_derive(CodamaType, attributes(codama))]
pub fn derive_codama_type(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// No-op stand-in for codama's instruction-enum marker (see the crate note).
#[proc_macro_derive(CodamaInstructions, attributes(codama))]
pub fn derive_codama_instructions(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// No-op stand-in for codama's PDA marker (see the crate note).
#[proc_macro_derive(CodamaPda, attributes(codama))]
pub fn derive_codama_pda(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
