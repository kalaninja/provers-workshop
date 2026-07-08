use pinocchio::{AccountView, Address, ProgramResult, cpi::Signer, error::ProgramError};
use pinocchio_system::instructions::CreateAccountAllowPrefund;

/// Returns exactly `N` mutable account views, rejecting missing or extra keys.
///
/// # Errors
///
/// Returns [`ProgramError::NotEnoughAccountKeys`] when fewer than `N` accounts
/// are supplied and [`ProgramError::InvalidArgument`] when more are supplied.
pub fn exact_mut<const N: usize>(
    accounts: &mut [AccountView],
) -> Result<&mut [AccountView; N], ProgramError> {
    let count = accounts.len();

    accounts.try_into().map_err(|_| {
        if count < N { ProgramError::NotEnoughAccountKeys } else { ProgramError::InvalidArgument }
    })
}

/// Requires that the account signed the transaction.
///
/// # Errors
///
/// Returns [`ProgramError::MissingRequiredSignature`] when the account is not
/// a signer.
pub fn require_signer(account: &AccountView) -> ProgramResult {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok(())
}

/// Requires that the account is writable.
///
/// # Errors
///
/// Returns [`ProgramError::InvalidAccountData`] when the account is not
/// writable.
pub fn require_writable(account: &AccountView) -> ProgramResult {
    if !account.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Requires that the account is owned by the given program.
///
/// # Errors
///
/// Returns [`ProgramError::InvalidAccountOwner`] when the account owner does
/// not match `program_id`.
pub fn require_owned_by(account: &AccountView, program_id: &Address) -> ProgramResult {
    if !account.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    Ok(())
}

/// System Program address — the `program("system")` `default_value`.
pub const SYSTEM_PROGRAM_ID: Address = pinocchio_system::id();

/// SPL Token program address — the `program("token")` `default_value`.
pub const TOKEN_PROGRAM_ID: Address =
    Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Rent sysvar address — the `sysvar("rent")` `default_value`.
pub const RENT_SYSVAR_ID: Address = pinocchio::sysvars::rent::RENT_ID;

/// Verifies that an account is the program at `expected_id`.
///
/// Generic over the program address so account parsers can run it for any
/// `default_value = program(...)` slot (supplying the matching `*_PROGRAM_ID`
/// constant) instead of needing a bespoke `require_*` per program.
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] when the account address does
/// not match `expected_id`.
pub fn require_program(program: &AccountView, expected_id: &Address) -> ProgramResult {
    if program.address() != expected_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

/// Verifies that an account is the sysvar at `expected_id`.
///
/// Generic over the sysvar address so account parsers can run it for any
/// `default_value = sysvar(...)` slot (supplying the matching `*_SYSVAR_ID`
/// constant) instead of needing a bespoke `require_*` per sysvar.
///
/// # Errors
///
/// Returns [`ProgramError::UnsupportedSysvar`] when the account address does
/// not match `expected_id`.
pub fn require_sysvar(sysvar: &AccountView, expected_id: &Address) -> ProgramResult {
    if sysvar.address() != expected_id {
        return Err(ProgramError::UnsupportedSysvar);
    }

    Ok(())
}

/// Creates a rent-funded PDA account and verifies its owner and data length.
///
/// The `CreateAccountAllowPrefund` CPI rejects a target that already carries
/// data or a foreign owner, but tolerates a pre-funded empty slot (topping up
/// only the rent deficit) — so a griefer cannot brick a derivable PDA by
/// sending it lamports before creation, and callers still get a freshly
/// initialized account or an error.
///
/// # Errors
///
/// Propagates any error from the System Program `CreateAccountAllowPrefund` CPI
/// and returns [`ProgramError::InvalidAccountOwner`] when the resulting account
/// owner does not match the requested value and
/// [`ProgramError::InvalidAccountData`] when the resulting data length does not
/// match the requested value.
pub fn create_pda_account(
    payer: &AccountView,
    account: &AccountView,
    data_len: usize,
    program_id: &Address,
    signer: Signer<'_, '_>,
) -> ProgramResult {
    CreateAccountAllowPrefund::with_minimum_balance(
        payer,
        account,
        data_len as u64,
        program_id,
        None,
    )?
    .invoke_signed(&[signer])?;

    require_owned_by(account, program_id)?;
    if account.data_len() != data_len {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use pinocchio::AccountView;
    use ticketing_macros::InstructionAccounts;

    use super::{RENT_SYSVAR_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID};

    #[test]
    fn identity_constants_pin_the_canonical_addresses() {
        assert_eq!(SYSTEM_PROGRAM_ID.to_string(), "11111111111111111111111111111111");
        assert_eq!(TOKEN_PROGRAM_ID.to_string(), "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        assert_eq!(RENT_SYSVAR_ID.to_string(), "SysvarRent111111111111111111111111111111111");
    }

    /// Compile-level guard for the `InstructionAccounts` identity arms: every
    /// fixed-address `default_value` the derive recognizes must resolve to a
    /// backing constant and check helper in this module, so a renamed or
    /// removed symbol fails this crate's build rather than the next user's.
    #[allow(dead_code)]
    #[derive(InstructionAccounts)]
    enum IdentityDefaults {
        #[codama(account(name = "system_program", default_value = program("system")))]
        #[codama(account(name = "token_program", default_value = program("token")))]
        #[codama(account(name = "rent", default_value = sysvar("rent")))]
        AllKnown,
    }

    #[test]
    fn every_recognized_identity_default_has_backing_symbols() {
        // Touch the generated parser and fields so the fixture is exercised
        // rather than dead code; the guard's value is that this file compiles.
        fn fields(accounts: AllKnownAccounts<'_>) -> [&mut AccountView; 3] {
            let AllKnownAccounts { system_program, token_program, rent } = accounts;
            [system_program, token_program, rent]
        }
        let _ = (AllKnownAccounts::parse, fields);
    }
}
