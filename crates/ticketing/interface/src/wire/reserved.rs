use bytemuck::{CheckedBitPattern, NoUninit, Zeroable};

/// Typed wire-format padding bytes.
///
/// Materialises the tail padding that `#[repr(C)]` would otherwise leave
/// invisible, so `#[derive(NoUninit)]` (which refuses any type with padding
/// bytes) accepts the parent struct. Canonical payloads must keep these
/// bytes zero; `CheckedBitPattern` rejects non-zero values on parse.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Zeroable)]
pub struct Reserved<const N: usize>([u8; N]);

impl<const N: usize> Reserved<N> {
    /// Canonical all-zero value.
    pub const ZERO: Self = Self([0; N]);
}

// SAFETY: `Reserved` is transparent over `[u8; N]` and has no padding bytes.
unsafe impl<const N: usize> NoUninit for Reserved<N> {}

// SAFETY: `Reserved<N>` is transparent over `[u8; N]`; the only accepted
// bit pattern is all-zero, enforcing that padding bytes stay zero on the
// wire so the parent struct's layout is fully defined.
unsafe impl<const N: usize> CheckedBitPattern for Reserved<N> {
    type Bits = [u8; N];

    fn is_valid_bit_pattern(bits: &Self::Bits) -> bool {
        bits.iter().all(|byte| *byte == 0)
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::checked::CheckedCastError;

    use super::Reserved;

    #[test]
    fn accepts_zero_reserved_bytes() {
        let bytes = [0_u8; 2];

        let reserved = bytemuck::checked::try_pod_read_unaligned::<Reserved<2>>(&bytes);

        assert_eq!(reserved, Ok(Reserved::ZERO));
    }

    #[test]
    fn rejects_nonzero_reserved_bytes() {
        let bytes = [0_u8, 1];

        let result = bytemuck::checked::try_pod_read_unaligned::<Reserved<2>>(&bytes);

        assert_eq!(result, Err(CheckedCastError::InvalidBitPattern));
    }

    #[test]
    fn reserved_zero_size_accepts_empty_slice() {
        let result = bytemuck::checked::try_pod_read_unaligned::<Reserved<0>>(&[]);

        assert_eq!(result, Ok(Reserved::ZERO));
    }

    #[test]
    fn reserved_zero_round_trips_through_bytes_of() {
        let reserved = Reserved::<8>::ZERO;
        let bytes = bytemuck::bytes_of(&reserved);

        assert_eq!(bytes, &[0_u8; 8]);
        assert_eq!(
            bytemuck::checked::try_pod_read_unaligned::<Reserved<8>>(bytes),
            Ok(Reserved::ZERO),
        );
    }

    #[test]
    fn reserved_rejects_any_nonzero_byte() {
        let single_nonzero = [0, 0, 0, 1, 0, 0, 0, 0];
        let all_nonzero = [0xFF_u8; 8];

        assert_eq!(
            bytemuck::checked::try_pod_read_unaligned::<Reserved<8>>(&single_nonzero),
            Err(CheckedCastError::InvalidBitPattern),
        );
        assert_eq!(
            bytemuck::checked::try_pod_read_unaligned::<Reserved<8>>(&all_nonzero),
            Err(CheckedCastError::InvalidBitPattern),
        );
    }
}
