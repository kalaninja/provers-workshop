//! Mollusk-SVM integration-test fixtures shared across the program and client
//! test suites.
//!
//! These helpers are program-agnostic: account fixtures, the Pinocchio rent
//! model, and result lookups. Program-specific seeders (state accounts, PDA
//! derivers, instruction builders) live with each test suite.
//!
//! Gated behind the `test-utils` feature so the host-only Mollusk dependencies
//! never reach normal or SBF builds.

use mollusk_svm::{
    Mollusk,
    program::keyed_account_for_system_program,
    result::{InstructionResult, ProgramResult},
};
use solana_account::Account;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;

/// Lamports loaded into a signer wallet for tests that fund the rent-exempt
/// allocation of a PDA.
pub const WALLET_LAMPORTS: u64 = 10_000_000_000;

/// Returns a Mollusk instance preloaded with the program identified by
/// `program_id` and the SBF artefact named `program_name` (without the `.so`
/// suffix). The system program is registered automatically by Mollusk.
#[must_use]
pub fn mollusk_for(program_id: &Pubkey, program_name: &str) -> Mollusk {
    Mollusk::new(program_id, program_name)
}

/// Returns an empty, system-owned, lamport-zero account suitable for use as a
/// PDA slot before the program creates it.
#[must_use]
pub fn empty_system_account() -> Account {
    Account {
        lamports: 0,
        data: Vec::new(),
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Returns a system-owned, fully funded user account with no data.
#[must_use]
pub fn funded_system_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: Vec::new(),
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Returns the `(Pubkey, Account)` entry for the System Program. Required in
/// the Mollusk accounts list whenever the program-under-test CPIs into the
/// System Program (e.g. via `CreateAccount` or `Transfer`). Mollusk pre-loads
/// the system program in its program cache, but the runtime still needs the
/// account in the transaction universe to invoke it.
#[must_use]
pub fn system_program_account() -> (Pubkey, Account) {
    keyed_account_for_system_program()
}

/// Returns the rent-exempt balance `CreateAccount::with_minimum_balance`
/// requests when a program CPIs inside the SVM.
///
/// Mollusk (since 0.13.1) seeds the rent sysvar in the post-SIMD-0194 shape:
/// `Rent::with_lamports_per_byte(6_960)` — the classic
/// `3480 * exemption_threshold 2.0` folded into one per-byte rate with the
/// threshold retired to `1.0`. Pinocchio's `Rent` reads exactly that leading
/// `u64` and computes `(data_len + 128) * lamports_per_byte`, so this helper
/// mirrors the same product and happy-path lamport assertions match what the
/// program's create-account CPI actually requests inside the test.
#[must_use]
pub fn rent_exempt_minimum(data_len: usize) -> u64 {
    const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;
    const LAMPORTS_PER_BYTE: u64 = 6_960;
    (data_len as u64 + ACCOUNT_STORAGE_OVERHEAD) * LAMPORTS_PER_BYTE
}

/// Looks up an account from the result's `resulting_accounts` by key.
#[must_use]
pub fn find_resulting_account(result: &InstructionResult, target: &Pubkey) -> Option<Account> {
    result
        .resulting_accounts
        .iter()
        .find_map(|(key, account)| (key == target).then(|| account.clone()))
}

/// Asserts the program ended with a `Failure` carrying the expected
/// `ProgramError`. Panics with a clear diagnostic on `Success` or runtime
/// errors that don't translate into a `ProgramError`.
///
/// # Panics
///
/// Panics when the result is not a `Failure` carrying `expected`.
#[allow(
    clippy::needless_pass_by_value,
    reason = "test helper called with ProgramError literals at every site"
)]
pub fn assert_program_error(actual: &ProgramResult, expected: ProgramError) {
    match actual {
        ProgramResult::Failure(err) => assert_eq!(*err, expected, "unexpected program error"),
        ProgramResult::Success => panic!("expected failure {expected:?}, got success"),
        ProgramResult::UnknownError(err) => panic!("unexpected runtime error: {err:?}"),
    }
}
