use core::ops::{Deref, DerefMut};

use bytemuck::{CheckedBitPattern, NoUninit};
use pinocchio::{
    AccountView, Address, ProgramResult,
    account::{Ref, RefMut},
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};

/// Fixed-layout account state: byte-level primitives only.
///
/// Implementations explicitly opt a type into byte-level account-state
/// operations backed by `bytemuck`. The account-level loaders / writers live
/// on [`PdaAccount`], which PDA-backed types implement explicitly alongside
/// [`CanonicalPda`].
pub trait StateAccount: CheckedBitPattern + NoUninit + Sized {
    /// Serialized byte length of the account state.
    const LEN: usize = size_of::<Self>();

    /// Casts immutable account data to this state type after layout checks.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when `data.len()` does not equal [`Self::LEN`] or
    /// when the bytes are not a valid bit pattern for `Self`.
    #[allow(clippy::result_unit_err)]
    fn cast(data: &[u8]) -> Result<&Self, ()> {
        if data.len() != Self::LEN {
            return Err(());
        }

        bytemuck::checked::try_from_bytes::<Self>(data).map_err(|_| ())
    }

    /// Casts mutable account data to this state type after layout checks.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when `data.len()` does not equal [`Self::LEN`] or
    /// when the bytes are not a valid bit pattern for `Self`.
    #[allow(clippy::result_unit_err)]
    fn cast_mut(data: &mut [u8]) -> Result<&mut Self, ()> {
        if data.len() != Self::LEN {
            return Err(());
        }

        bytemuck::checked::try_from_bytes_mut::<Self>(data).map_err(|_| ())
    }

    /// Reads immutable account data as this state type after layout
    /// validation.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidAccountData`] when the byte layout does
    /// not match `Self`.
    fn read_from(data: &[u8]) -> Result<&Self, ProgramError> {
        Self::cast(data).map_err(|()| ProgramError::InvalidAccountData)
    }

    /// Reads mutable account data as this state type after layout validation.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidAccountData`] when the byte layout does
    /// not match `Self`.
    fn read_from_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        Self::cast_mut(data).map_err(|()| ProgramError::InvalidAccountData)
    }

    /// Writes the state into an account data buffer.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::AccountDataTooSmall`] when `data.len()` does
    /// not equal [`Self::LEN`].
    fn write_to(&self, data: &mut [u8]) -> ProgramResult {
        if data.len() != Self::LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }

        data.copy_from_slice(bytemuck::bytes_of(self));

        Ok(())
    }
}

/// State that records the slot at which it was last mutated.
///
/// Implementations expose the stored slot via [`Self::current_slot`] and
/// allow it to be overwritten via [`Self::set_current_slot`]. Production
/// mutations should go through [`PdaAccount::mutate_account_at`], which
/// returns a [`SlotGuard`] that validates monotonicity up front and
/// writes the pinned slot back on drop.
pub trait SlotTracked: Sized {
    /// Returns the slot recorded on the last mutation.
    fn current_slot(&self) -> u64;

    /// Overwrites the slot recorded on the last mutation. Prefer
    /// [`PdaAccount::mutate_account_at`] over calling this directly so the
    /// monotonicity check is not bypassed.
    fn set_current_slot(&mut self, slot: u64);
}

/// RAII guard returned by [`PdaAccount::mutate_account_at`]. Wraps the
/// account's [`RefMut`] borrow plus a pinned slot. On drop the pinned
/// slot is written back unconditionally, so the bytes flushed to the
/// account always carry that value (any mid-scope `set_current_slot`
/// calls are overwritten before the borrow releases). Field access goes
/// through [`Deref`] / [`DerefMut`] to the underlying state.
///
/// The slot is written unconditionally on drop, including when the
/// guarded block exits via `?` or `return Err(...)`. This is safe in the
/// ticketing program because instruction handlers do not commit account
/// data when they return `Err`, so the partial in-memory bump is
/// discarded.
pub struct SlotGuard<'a, T: SlotTracked> {
    inner: RefMut<'a, T>,
    slot: u64,
}

impl<T: SlotTracked> Deref for SlotGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: SlotTracked> DerefMut for SlotGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: SlotTracked> Drop for SlotGuard<'_, T> {
    fn drop(&mut self) {
        self.inner.set_current_slot(self.slot);
    }
}

/// State that lives at a canonical program-derived address whose seed
/// material is fully recoverable from `self`.
///
/// Pure PDA derivation interface — does not depend on [`StateAccount`].
/// The const-generic `VARIANT_SEEDS` is the entity-specific seed slot count;
/// the entity declares it at the `impl` block (`impl CanonicalPda<1, 2> for
/// Event`), so the returned `[&[u8]; SEEDS]` from [`Self::pda_seed`] feeds
/// directly into const-generic [`Address::derive_address`] without any
/// runtime dispatch.
///
/// Two responsibilities, two top-level methods (both default-impl'd):
///
/// - [`Self::canonical_address`] *verifies* an account at read time. It
///   recomputes the PDA from `self`'s stored seed fields and stored bump using
///   [`Address::derive_address`] with `Some(self.get_bump())` — single
///   in-program SHA-256, no syscall, no bump search. Comparing the result to
///   the account's actual address transitively verifies the stored bump (a
///   wrong bump cannot produce the same hash).
/// - [`Self::derive_address`] *finds* the canonical PDA from raw seed material.
///   CREATE flows use this to obtain the canonical bump for PDA signing before
///   the account exists.
///
/// Types that additionally implement [`StateAccount`] should declare
/// [`PdaAccount`] explicitly to receive the account-level loaders,
/// writers, and creators.
pub trait CanonicalPda<const VARIANT_SEEDS: usize, const SEEDS: usize> {
    /// Seed material the canonical PDA is derived from. `()` for
    /// singleton state; a single value or tuple for instance-keyed
    /// state.
    type Seeds;

    /// Wire-stable PDA namespace prefix. Auto-prepended to every seed
    /// array the trait constructs via [`Self::pda_seed`]; impls do not
    /// include it in [`Self::variant_seeds`].
    const NAMESPACE: &'static [u8];

    /// Returns the canonical-PDA bump stored on the entity.
    fn get_bump(&self) -> u8;

    /// Reconstructs the canonical seed material from `self`'s stored
    /// fields.
    fn get_seeds(&self) -> Self::Seeds;

    /// Returns the entity-specific seed slices in derivation order. The
    /// namespace is not included — the trait prepends it automatically
    /// via [`Self::pda_seed`].
    fn variant_seeds(seeds: &Self::Seeds) -> [&[u8]; VARIANT_SEEDS];

    /// Combines [`Self::NAMESPACE`] with [`Self::variant_seeds`] into the
    /// full canonical PDA seed array.
    ///
    /// `SEEDS` is a separate const generic because stable Rust can't
    /// express `[&[u8]; VARIANT_SEEDS + 1]` from a single const generic
    /// (`generic_const_exprs` is unstable). Convention: every impl
    /// declares `SEEDS == VARIANT_SEEDS + 1`; we enforce that at
    /// monomorphization time below.
    fn pda_seed(seeds: &Self::Seeds) -> [&[u8]; SEEDS] {
        const {
            assert!(
                SEEDS == VARIANT_SEEDS + 1,
                "CanonicalPda: SEEDS must equal VARIANT_SEEDS + 1 \
                 (one namespace slot prepended to the variant_seeds slots).",
            );
        }
        let variant = Self::variant_seeds(seeds);
        core::array::from_fn(|i| if i == 0 { Self::NAMESPACE } else { variant[i - 1] })
    }

    /// Recomputes the canonical PDA from `self`'s stored fields via the
    /// single-SHA256 cheap path ([`Address::derive_address`]).
    fn canonical_address(&self, program_id: &Address) -> Address {
        let seeds = self.get_seeds();
        Address::derive_address(&Self::pda_seed(&seeds), Some(self.get_bump()), program_id)
    }

    /// Locates the canonical PDA and its bump from raw seed material via
    /// [`Address::try_find_program_address`]. Used by CREATE flows.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidSeeds`] when no PDA can be derived
    /// from the supplied seeds.
    fn derive_address(
        program_id: &Address,
        seeds: &Self::Seeds,
    ) -> Result<(Address, u8), ProgramError> {
        Address::try_find_program_address(&Self::pda_seed(seeds), program_id)
            .ok_or(ProgramError::InvalidSeeds)
    }
}

/// Account-level loaders / writers / creators for PDA-backed state.
///
/// Combines [`StateAccount`]'s byte-level layout with [`CanonicalPda`]'s
/// PDA derivation, and exposes the operations that touch an
/// [`AccountView`]: reading an existing account into a typed `Ref<Self>`
/// with full canonical-PDA validation, opening a validated mutation borrow,
/// writing initial state after PDA creation, and opening the System-Program
/// CPI to create a new PDA.
///
/// `SIGNER_SEEDS` is the size of the signer-seed buffer produced by
/// [`Self::signer_seed_buf`] and consumed by [`Self::create_pda_account`]
/// and external CPI sites. By construction it equals `SEEDS + 1` (the
/// PDA seeds plus the bump byte) — enforced at compile time via a
/// `const { assert!(...) }` block inside [`Self::signer_seed_buf`]. We
/// declare it as a separate const generic because stable Rust can't
/// express `[Seed; SEEDS + 1]` from a single const generic
/// (`generic_const_exprs` is unstable).
///
/// Each entity declares its own empty `impl PdaAccount<VARIANT_SEEDS, SEEDS,
/// SIGNER_SEEDS>` — there is no blanket impl, since const generics with the
/// convention `SIGNER_SEEDS = SEEDS + 1` can't be deduced automatically.
pub trait PdaAccount<const VARIANT_SEEDS: usize, const SEEDS: usize, const SIGNER_SEEDS: usize>:
    StateAccount + CanonicalPda<VARIANT_SEEDS, SEEDS>
{
    /// Builds the `[Seed; SIGNER_SEEDS]` buffer that `Signer::from(&buf)`
    /// consumes when invoking a CPI signed by this canonical PDA. Slots
    /// `0..SEEDS` mirror [`CanonicalPda::pda_seed`]; slot `SEEDS` points
    /// at `bump`. The caller owns the `bump` storage so the returned
    /// array's borrow chain is rooted in the caller's frame — `Signer<'_>`
    /// cannot be returned directly because it would borrow from a
    /// stack-local inside this function, and `Seed` itself never owns
    /// its bytes.
    fn signer_seed_buf<'a>(seeds: &'a Self::Seeds, bump: &'a u8) -> [Seed<'a>; SIGNER_SEEDS] {
        const {
            assert!(
                SIGNER_SEEDS == SEEDS + 1,
                "PdaAccount: SIGNER_SEEDS must equal SEEDS + 1 \
                 (one bump byte appended to the pda_seed slots).",
            );
        }
        let seed_slices = Self::pda_seed(seeds);
        core::array::from_fn(|i| {
            if i < SEEDS {
                Seed::from(seed_slices[i])
            } else {
                Seed::from(core::slice::from_ref(bump))
            }
        })
    }

    /// Creates the canonical PDA account at the address derived from
    /// `seeds`, signed by the program with the canonical seeds + bump.
    /// Returns the bump that was used.
    ///
    /// `account` must be the empty system-owned account at the canonical
    /// PDA address. After the call the program is the owner and the data
    /// area has [`StateAccount::LEN`] bytes ready for
    /// [`Self::write_to_account`].
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidSeeds`] when `account.address()`
    /// does not match the canonical PDA. Propagates any error from the
    /// `CreateAccount` CPI and the post-creation owner / length checks
    /// in [`crate::program::accounts::create_pda_account`].
    fn create_pda_account(
        program_id: &Address,
        payer: &AccountView,
        account: &AccountView,
        seeds: &Self::Seeds,
    ) -> Result<u8, ProgramError> {
        let (expected_address, bump) = Self::derive_address(program_id, seeds)?;
        if account.address() != &expected_address {
            return Err(ProgramError::InvalidSeeds);
        }
        let buf = Self::signer_seed_buf(seeds, &bump);

        crate::program::accounts::create_pda_account(
            payer,
            account,
            Self::LEN,
            program_id,
            Signer::from(&buf),
        )?;
        Ok(bump)
    }

    /// Reads an account view's data as this state type after enforcing
    /// program ownership, the fixed layout length, and a canonical-PDA
    /// check that binds the entity's seed fields to the account's actual
    /// address.
    ///
    /// This loader validates byte layout and the canonical PDA. Domain
    /// invariants are the responsibility of the account type's constructors
    /// and typed mutators.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidAccountOwner`] when the account is
    /// not owned by `program_id`, [`ProgramError::InvalidAccountData`]
    /// when the data length does not match [`StateAccount::LEN`],
    /// [`ProgramError::AccountBorrowFailed`] when the account is already
    /// mutably borrowed, and [`ProgramError::InvalidSeeds`] when the
    /// account's address does not match the canonical PDA derived from
    /// the entity's own seed material.
    fn read_from_account<'a>(
        program_id: &Address,
        account: &'a AccountView,
    ) -> Result<Ref<'a, Self>, ProgramError> {
        if !account.owned_by(program_id) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let account_address = *account.address();
        let data = account.try_borrow()?;
        let state = Ref::try_map(data, |bytes| Self::read_from(bytes)).map_err(|(_, e)| e)?;
        if account_address != state.canonical_address(program_id) {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(state)
    }

    /// Writes this state into an account view's data buffer.
    ///
    /// Writes occur only in CREATE flows immediately after
    /// [`Self::create_pda_account`] has placed the account at the canonical
    /// PDA, so the address is correct by construction and no canonical
    /// check happens here.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::AccountBorrowFailed`] when the account is
    /// already borrowed and [`ProgramError::AccountDataTooSmall`] when the
    /// data length does not equal [`StateAccount::LEN`].
    fn write_to_account(&self, account: &mut AccountView) -> ProgramResult {
        let mut data = account.try_borrow_mut()?;
        self.write_to(&mut data)
    }

    /// Opens a slot-pinned mutation borrow. The returned [`SlotGuard`]
    /// auto-derefs to `Self` for field access and writes the pinned
    /// `slot` back on drop. Available only for [`SlotTracked`] types.
    ///
    /// Pre-mutation state is not re-validated; post-mutation invariants
    /// are the responsibility of the typed mutator methods called through
    /// the guard.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidAccountOwner`] when the account is
    /// not owned by `program_id`, [`ProgramError::InvalidAccountData`]
    /// when the data length does not match [`StateAccount::LEN`],
    /// [`ProgramError::AccountBorrowFailed`] when the account is already
    /// borrowed in any form, [`ProgramError::InvalidSeeds`] when the
    /// account's address does not match the canonical PDA, and
    /// [`ProgramError::InvalidArgument`] when `slot` is below the stored
    /// slot.
    fn mutate_account_at<'a>(
        program_id: &Address,
        account: &'a mut AccountView,
        slot: u64,
    ) -> Result<SlotGuard<'a, Self>, ProgramError>
    where
        Self: SlotTracked,
    {
        if !account.owned_by(program_id) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let account_address = *account.address();
        let data = account.try_borrow_mut()?;
        let inner =
            RefMut::try_map(data, |bytes| Self::read_from_mut(bytes)).map_err(|(_, e)| e)?;
        if account_address != inner.canonical_address(program_id) {
            return Err(ProgramError::InvalidSeeds);
        }
        if slot < inner.current_slot() {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(SlotGuard { inner, slot })
    }

    /// As [`Self::mutate_account_at`] but pins to the current slot from
    /// the Solana clock sysvar.
    ///
    /// # Errors
    ///
    /// In addition to the errors from [`Self::mutate_account_at`],
    /// propagates any error from reading the clock sysvar.
    fn mutate_account_at_current<'a>(
        program_id: &Address,
        account: &'a mut AccountView,
    ) -> Result<SlotGuard<'a, Self>, ProgramError>
    where
        Self: SlotTracked,
    {
        Self::mutate_account_at(program_id, account, Clock::get()?.slot)
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::{CheckedBitPattern, NoUninit};
    use pinocchio::{Address, error::ProgramError};

    use super::{CanonicalPda, SlotTracked, StateAccount};

    #[repr(C)]
    #[derive(Clone, Copy, NoUninit, CheckedBitPattern, PartialEq, Debug)]
    struct Fixture {
        slot: u64,
        value: u64,
    }

    impl StateAccount for Fixture {}

    impl SlotTracked for Fixture {
        fn current_slot(&self) -> u64 {
            self.slot
        }
        fn set_current_slot(&mut self, slot: u64) {
            self.slot = slot;
        }
    }

    fn fixture() -> Fixture {
        Fixture { slot: 10, value: 42 }
    }

    #[test]
    fn cast_accepts_valid_layout() {
        let fixture = fixture();
        let bytes = bytemuck::bytes_of(&fixture);

        let view = Fixture::cast(bytes).expect("cast succeeds");

        assert_eq!(view, &fixture);
    }

    #[test]
    fn cast_rejects_undersized_length() {
        let bytes = [0_u8; Fixture::LEN - 1];

        assert_eq!(Fixture::cast(&bytes), Err(()));
    }

    #[test]
    fn cast_rejects_oversized_length() {
        let bytes = [0_u8; Fixture::LEN + 1];

        assert_eq!(Fixture::cast(&bytes), Err(()));
    }

    #[test]
    fn cast_mut_returns_mutable_ref() {
        let mut bytes = bytemuck::bytes_of(&fixture()).to_vec();

        let view = Fixture::cast_mut(&mut bytes).expect("cast_mut succeeds");
        view.value = 99;

        let reread = Fixture::cast(&bytes).expect("re-cast succeeds");
        assert_eq!(reread.value, 99);
    }

    #[test]
    fn cast_mut_rejects_wrong_length() {
        let mut bytes = [0_u8; Fixture::LEN - 1];

        assert_eq!(Fixture::cast_mut(&mut bytes), Err(()));
    }

    #[test]
    fn read_from_accepts_valid_state() {
        let fixture = fixture();
        let bytes = bytemuck::bytes_of(&fixture);

        let view = Fixture::read_from(bytes).expect("read_from succeeds");

        assert_eq!(view, &fixture);
    }

    #[test]
    fn read_from_rejects_wrong_length() {
        let bytes = [0_u8; Fixture::LEN - 1];

        assert_eq!(Fixture::read_from(&bytes), Err(ProgramError::InvalidAccountData));
    }

    #[test]
    fn read_from_mut_returns_mutable_ref() {
        let mut bytes = bytemuck::bytes_of(&fixture()).to_vec();

        let view = Fixture::read_from_mut(&mut bytes).expect("read_from_mut succeeds");
        view.value = 77;

        let reread = Fixture::read_from(&bytes).expect("re-read succeeds");
        assert_eq!(reread.value, 77);
    }

    #[test]
    fn write_to_round_trips() {
        let fixture = fixture();
        let mut buffer = vec![0_u8; Fixture::LEN];

        fixture.write_to(&mut buffer).expect("write_to succeeds");

        let reread = Fixture::read_from(&buffer).expect("read_from succeeds");
        assert_eq!(reread, &fixture);
    }

    #[test]
    fn write_to_rejects_undersized_buffer() {
        let fixture = fixture();
        let mut buffer = vec![0_u8; Fixture::LEN - 1];

        assert_eq!(fixture.write_to(&mut buffer), Err(ProgramError::AccountDataTooSmall));
    }

    #[test]
    fn write_to_rejects_oversized_buffer() {
        let fixture = fixture();
        let mut buffer = vec![0_u8; Fixture::LEN + 1];

        assert_eq!(fixture.write_to(&mut buffer), Err(ProgramError::AccountDataTooSmall));
    }

    #[test]
    fn slot_tracked_get_set_roundtrip() {
        let mut fixture = fixture();

        assert_eq!(fixture.current_slot(), 10);
        fixture.set_current_slot(99);
        assert_eq!(fixture.current_slot(), 99);
    }

    #[repr(C)]
    #[derive(Clone, Copy, NoUninit, CheckedBitPattern, PartialEq, Debug)]
    struct PdaFixture {
        key: [u8; 32],
        bump: u8,
    }

    impl StateAccount for PdaFixture {}

    impl CanonicalPda<1, 2> for PdaFixture {
        type Seeds = [u8; 32];
        const NAMESPACE: &'static [u8] = b"test-pda";

        fn get_bump(&self) -> u8 {
            self.bump
        }

        fn get_seeds(&self) -> [u8; 32] {
            self.key
        }

        fn variant_seeds(seeds: &[u8; 32]) -> [&[u8]; 1] {
            [seeds.as_slice()]
        }
    }

    fn test_program_id() -> Address {
        Address::new_from_array([7; 32])
    }

    #[test]
    fn pda_seed_prepends_namespace() {
        let key = [3_u8; 32];

        let seeds = PdaFixture::pda_seed(&key);

        assert_eq!(seeds[0], PdaFixture::NAMESPACE);
        assert_eq!(seeds[1], &key[..]);
    }

    #[test]
    fn derive_address_round_trips_with_canonical_address() {
        let program_id = test_program_id();
        let key = [3_u8; 32];

        let (address, bump) = PdaFixture::derive_address(&program_id, &key).expect("PDA derives");

        let fixture = PdaFixture { key, bump };
        assert_eq!(fixture.canonical_address(&program_id), address);
    }

    #[test]
    fn canonical_address_changes_with_seeds() {
        let program_id = test_program_id();

        let (address_a, bump_a) =
            PdaFixture::derive_address(&program_id, &[3_u8; 32]).expect("PDA derives");
        let (address_b, bump_b) =
            PdaFixture::derive_address(&program_id, &[4_u8; 32]).expect("PDA derives");

        assert_ne!(address_a, address_b);

        let fixture_a = PdaFixture { key: [3_u8; 32], bump: bump_a };
        let fixture_b = PdaFixture { key: [4_u8; 32], bump: bump_b };
        assert_eq!(fixture_a.canonical_address(&program_id), address_a);
        assert_eq!(fixture_b.canonical_address(&program_id), address_b);
    }
}
