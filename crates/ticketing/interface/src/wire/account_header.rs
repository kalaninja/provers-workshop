use core::{fmt, marker::PhantomData};

use bytemuck::{CheckedBitPattern, NoUninit};

use super::{DISCRIMINATOR_BYTES, Discriminator};

/// Number of bytes in a fixed account header.
pub const ACCOUNT_HEADER_BYTES: usize = DISCRIMINATOR_BYTES + 1;

/// Defines the fixed discriminator and schema version for an account header.
pub trait AccountHeaderSpec: 'static {
    /// Fixed account discriminator stored at the start of the header.
    const DISCRIMINATOR: Discriminator;
    /// Account schema version stored after the discriminator.
    const VERSION: u8;
}

/// Common fixed-size prefix for a specific versioned account state.
#[repr(transparent)]
pub struct AccountHeader<T>([u8; ACCOUNT_HEADER_BYTES], PhantomData<fn() -> T>);

impl<T> Clone for AccountHeader<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for AccountHeader<T> {}

impl<T> fmt::Debug for AccountHeader<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("AccountHeader").field(&self.0).finish()
    }
}

impl<T> Eq for AccountHeader<T> {}

impl<T> PartialEq for AccountHeader<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: AccountHeaderSpec> Default for AccountHeader<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: AccountHeaderSpec> AccountHeader<T> {
    const BYTES: [u8; ACCOUNT_HEADER_BYTES] = {
        let [d0, d1, d2, d3, d4, d5, d6] = T::DISCRIMINATOR;
        let version = T::VERSION;

        [d0, d1, d2, d3, d4, d5, d6, version]
    };

    /// Fixed account discriminator for this header type.
    pub const DISCRIMINATOR: Discriminator = T::DISCRIMINATOR;
    /// Serialized byte length of an account header.
    pub const LEN: usize = ACCOUNT_HEADER_BYTES;
    /// Schema version for this header type.
    pub const VERSION: u8 = T::VERSION;

    /// Creates the canonical header bytes for this account type.
    #[must_use]
    pub const fn new() -> Self {
        Self(Self::BYTES, PhantomData)
    }

    /// Returns this header type's account discriminator.
    #[must_use]
    pub const fn discriminator(&self) -> Discriminator {
        Self::DISCRIMINATOR
    }

    /// Returns this header type's schema version.
    #[must_use]
    pub const fn version(&self) -> u8 {
        Self::VERSION
    }

    /// Returns the serialized header bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; ACCOUNT_HEADER_BYTES] {
        &self.0
    }
}

// SAFETY: `AccountHeader` is transparent over `[u8; ACCOUNT_HEADER_BYTES]` and
// has no padding bytes.
unsafe impl<T: 'static> NoUninit for AccountHeader<T> {}

// SAFETY: `AccountHeader<T>` has the same layout as `[u8;
// ACCOUNT_HEADER_BYTES]`; the only accepted bit pattern is the account header
// encoded by `T`.
unsafe impl<T: AccountHeaderSpec> CheckedBitPattern for AccountHeader<T> {
    type Bits = [u8; ACCOUNT_HEADER_BYTES];

    fn is_valid_bit_pattern(bits: &Self::Bits) -> bool {
        bits == &AccountHeader::<T>::BYTES
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, size_of};

    use super::{ACCOUNT_HEADER_BYTES, AccountHeader, AccountHeaderSpec};

    struct TestHeader;

    impl AccountHeaderSpec for TestHeader {
        const DISCRIMINATOR: [u8; 7] = *b"TKTTEST";
        const VERSION: u8 = 3;
    }

    #[test]
    fn account_header_len_matches_wire_layout() {
        assert_eq!(AccountHeader::<TestHeader>::LEN, 8);
        assert_eq!(ACCOUNT_HEADER_BYTES, size_of::<AccountHeader<TestHeader>>());
        assert_eq!(align_of::<AccountHeader<TestHeader>>(), 1);
    }

    #[test]
    fn account_header_bytes_match_wire_layout() {
        let header = AccountHeader::<TestHeader>::new();

        let bytes = bytemuck::bytes_of(&header);

        assert_eq!(bytes, b"TKTTEST\x03");
        assert_eq!(header.as_bytes(), b"TKTTEST\x03");
    }

    #[test]
    fn account_header_exposes_discriminator_and_version() {
        let header = AccountHeader::<TestHeader>::new();

        assert_eq!(header.discriminator(), *b"TKTTEST");
        assert_eq!(header.version(), 3);
    }

    #[test]
    fn account_header_accepts_matching_bytes() {
        let bytes = *b"TKTTEST\x03";

        let header = bytemuck::checked::try_pod_read_unaligned::<AccountHeader<TestHeader>>(&bytes);

        assert_eq!(header, Ok(AccountHeader::new()));
    }

    #[test]
    fn account_header_rejects_version_mismatch() {
        let bytes = *b"TKTTEST\x04";

        let header = bytemuck::checked::try_pod_read_unaligned::<AccountHeader<TestHeader>>(&bytes);

        assert_eq!(header, Err(bytemuck::checked::CheckedCastError::InvalidBitPattern));
    }

    #[test]
    fn account_header_rejects_discriminator_mismatch() {
        let bytes = *b"XKTTEST\x03";

        let header = bytemuck::checked::try_pod_read_unaligned::<AccountHeader<TestHeader>>(&bytes);

        assert_eq!(header, Err(bytemuck::checked::CheckedCastError::InvalidBitPattern));
    }
}
