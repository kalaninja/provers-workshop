use bytemuck::{CheckedBitPattern, NoUninit, Pod, Zeroable};

/// Error returned when constructing or reading a bounded wire string fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedStringError {
    /// The provided or stored length exceeds the bounded string capacity.
    TooLong,
}

/// Raw bit-pattern carrier for `BoundedString` checked casts.
///
/// This type is public only because `CheckedBitPattern::Bits` is part of the
/// public trait implementation for `BoundedString`. It is an internal wire
/// layout detail and should not be used as a consumer-facing API.
#[doc(hidden)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable)]
pub struct BoundedStringBits<const N: usize> {
    len: u16,
    bytes: [u8; N],
}

/// Fixed-capacity byte string for stable wire and account layouts.
///
/// `BoundedString` does not enforce UTF-8. Callers that require text must
/// validate UTF-8 at the relevant instruction or client boundary.
///
/// Checked wire-layout traits are implemented only for supported capacities
/// whose `repr(C)` layout has no tail padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedString<const N: usize> {
    /// Number of active bytes in `bytes`.
    len: u16,
    /// Fixed-capacity byte storage. Bytes after `len` are padding.
    bytes: [u8; N],
}

impl<const N: usize> BoundedString<N> {
    /// Fails compilation if the bounded string capacity cannot fit in `len`.
    ///
    /// # Panics
    ///
    /// Panics at compile time when `N` exceeds [`u16::MAX`].
    pub const fn assert_capacity_fits_u16() {
        assert!(N <= u16::MAX as usize, "BoundedString capacity exceeds u16::MAX");
    }

    /// Creates an empty bounded string with zeroed storage.
    #[must_use]
    pub const fn empty() -> Self {
        Self::assert_capacity_fits_u16();

        Self { len: 0, bytes: [0; N] }
    }

    /// Copies `bytes` into fixed-capacity storage.
    ///
    /// # Errors
    ///
    /// Returns [`BoundedStringError::TooLong`] when `bytes.len()` exceeds `N`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BoundedStringError> {
        Self::assert_capacity_fits_u16();

        if bytes.len() > N {
            return Err(BoundedStringError::TooLong);
        }

        let mut value = Self::empty();
        value.len = bytes.len() as u16;
        value.bytes[..bytes.len()].copy_from_slice(bytes);

        Ok(value)
    }

    /// Returns the number of active bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns true when no active bytes are stored.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the active byte slice.
    ///
    /// # Errors
    ///
    /// Returns [`BoundedStringError::TooLong`] when the stored `len` is larger
    /// than the fixed storage capacity.
    pub fn as_bytes(&self) -> Result<&[u8], BoundedStringError> {
        self.bytes.get(..self.len()).ok_or(BoundedStringError::TooLong)
    }
}

impl<const N: usize> Default for BoundedString<N> {
    fn default() -> Self {
        Self::empty()
    }
}

macro_rules! impl_bounded_string_wire_traits {
    ($($capacity:literal),+ $(,)?) => {
        $(
            // SAFETY: These supported capacities are even and fit in `u16`, so
            // the `repr(C)` layout is exactly a `u16` followed by `$capacity`
            // bytes with no internal or tail padding.
            unsafe impl Pod for BoundedStringBits<$capacity> {}

            // SAFETY: All-zero bytes produce `len = 0` and zeroed storage.
            unsafe impl Zeroable for BoundedString<$capacity> {}

            // SAFETY: These supported capacities are even and fit in `u16`, so
            // the `repr(C)` layout is exactly a `u16` followed by `$capacity`
            // bytes with no internal or tail padding.
            unsafe impl NoUninit for BoundedString<$capacity> {}

            // SAFETY: `BoundedString<$capacity>` and
            // `BoundedStringBits<$capacity>` have the same `repr(C)` layout.
            // The accepted bit patterns are canonical bounded strings: `len`
            // must fit within the fixed storage and inactive padding bytes must
            // be zero.
            unsafe impl CheckedBitPattern for BoundedString<$capacity> {
                type Bits = BoundedStringBits<$capacity>;

                fn is_valid_bit_pattern(bits: &Self::Bits) -> bool {
                    let len = bits.len as usize;

                    len <= $capacity && super::all_zero(&bits.bytes[len..])
                }
            }
        )+
    };
}

impl_bounded_string_wire_traits!(64);

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::{BoundedString, BoundedStringBits, BoundedStringError};

    #[test]
    fn bounded_string_stores_len_and_bytes() {
        let value = BoundedString::<8>::from_bytes(b"rustconf").expect("bounded string fits");

        assert_eq!(value.len(), 8);
        assert_eq!(value.as_bytes(), Ok(&b"rustconf"[..]));
    }

    #[test]
    fn bounded_string_accepts_exactly_capacity() {
        let value = BoundedString::<4>::from_bytes(b"expo").expect("full capacity fits");

        assert_eq!(value.len(), 4);
        assert!(!value.is_empty());
        assert_eq!(value.as_bytes(), Ok(&b"expo"[..]));
    }

    #[test]
    fn bounded_string_empty_round_trips() {
        let value = BoundedString::<8>::empty();

        assert_eq!(value.len(), 0);
        assert!(value.is_empty());
        assert_eq!(value.as_bytes(), Ok(&[][..]));
    }

    #[test]
    fn bounded_string_rejects_oversized_input() {
        let result = BoundedString::<4>::from_bytes(b"festival");

        assert_eq!(result, Err(BoundedStringError::TooLong));
    }

    #[test]
    fn bounded_string_supported_wire_layouts_have_no_padding() {
        fn assert_checked<T: bytemuck::CheckedBitPattern + bytemuck::NoUninit>() {}

        BoundedString::<64>::assert_capacity_fits_u16();
        assert_checked::<BoundedString<64>>();
        assert_eq!(align_of::<BoundedString<64>>(), 2);
        assert_eq!(size_of::<BoundedString<64>>(), 66);
    }

    #[test]
    fn bounded_string_bytes_match_wire_layout() {
        let value = BoundedString::<64>::from_bytes(b"rust-summit-2026").expect("name fits");

        let bytes = bytemuck::bytes_of(&value);

        assert_eq!(bytes.len(), 66);
        assert_eq!(&bytes[..2], &16_u16.to_le_bytes());
        assert_eq!(&bytes[2..18], b"rust-summit-2026");
        assert_eq!(&bytes[18..], [0; 48]);
    }

    #[test]
    fn bounded_string_checked_pattern_accepts_canonical_bytes() {
        let bytes = BoundedStringBits::<64> {
            len: 3,
            bytes: {
                let mut bytes = [0; 64];
                bytes[..3].copy_from_slice(b"abc");
                bytes
            },
        };

        let result = bytemuck::checked::try_pod_read_unaligned::<BoundedString<64>>(
            bytemuck::bytes_of(&bytes),
        );

        assert_eq!(result.expect("canonical bytes parse").as_bytes(), Ok(&b"abc"[..]));
    }

    #[test]
    fn bounded_string_checked_pattern_rejects_oversized_len() {
        let bytes = BoundedStringBits::<64> { len: 65, bytes: [0; 64] };

        let result = bytemuck::checked::try_pod_read_unaligned::<BoundedString<64>>(
            bytemuck::bytes_of(&bytes),
        );

        assert_eq!(result, Err(bytemuck::checked::CheckedCastError::InvalidBitPattern));
    }

    #[test]
    fn bounded_string_checked_pattern_rejects_nonzero_padding() {
        let mut bytes = BoundedStringBits::<64> { len: 3, bytes: [0; 64] };
        bytes.bytes[..3].copy_from_slice(b"abc");
        bytes.bytes[3] = 1;

        let result = bytemuck::checked::try_pod_read_unaligned::<BoundedString<64>>(
            bytemuck::bytes_of(&bytes),
        );

        assert_eq!(result, Err(bytemuck::checked::CheckedCastError::InvalidBitPattern));
    }

    #[test]
    fn bounded_string_checked_pattern_rejects_nonzero_trailing_padding() {
        let mut bytes = BoundedStringBits::<64> { len: 3, bytes: [0; 64] };
        bytes.bytes[..3].copy_from_slice(b"abc");
        bytes.bytes[63] = 1;

        let result = bytemuck::checked::try_pod_read_unaligned::<BoundedString<64>>(
            bytemuck::bytes_of(&bytes),
        );

        assert_eq!(result, Err(bytemuck::checked::CheckedCastError::InvalidBitPattern));
    }
}
