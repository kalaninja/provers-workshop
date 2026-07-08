use core::mem::MaybeUninit;

use bytemuck::{CheckedBitPattern, NoUninit, Zeroable};
use pinocchio::error::ProgramError;

/// Fixed-layout instruction payload that can be checked and decoded from bytes.
pub trait InstructionArgs: NoUninit + CheckedBitPattern + Sized {
    /// Exact payload length expected after the one-byte instruction tag.
    const LEN: usize = size_of::<Self>();

    /// Number of trailing `Reserved<N>` bytes the parser will accept as
    /// omitted.
    ///
    /// When the last `TRAILING_RESERVED` bytes of [`Self::LEN`] correspond to a
    /// `Reserved<N>` field that is hidden from the IDL (alignment-driven tail
    /// padding), foreign clients generated from the IDL will not emit them.
    /// [`Self::parse`] then accepts a payload of `LEN - TRAILING_RESERVED`
    /// bytes and zero-fills the missing tail before running the normal
    /// `CheckedBitPattern` validation. The default `0` keeps the parser strict
    /// for args without trailing reserved padding.
    ///
    /// Implementors that set a non-zero value must also assert at compile time
    /// that the trailing `Reserved` field sits at `LEN - TRAILING_RESERVED`
    /// (see `EventCreateArgs`), so a later field reorder or resize cannot
    /// silently let the parser zero-fill a real field.
    const TRAILING_RESERVED: usize = 0;

    /// Parses instruction payload bytes into the typed wire-format arguments.
    ///
    /// Accepts a payload of [`Self::LEN`] bytes (the canonical struct layout)
    /// or, when [`Self::TRAILING_RESERVED`] is non-zero, a payload of
    /// `LEN - TRAILING_RESERVED` bytes that omits the trailing reserved
    /// padding. In the latter case the parser zero-extends the payload before
    /// validation, so the `Reserved<N>` all-zero invariant is enforced exactly
    /// as it is for the full-length form.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError::InvalidInstructionData`] when `data.len()` does
    /// not match either accepted length, or when the (possibly zero-extended)
    /// bytes are not a valid bit pattern for `Self`.
    fn parse(data: &[u8]) -> Result<Self, ProgramError> {
        let len = data.len();
        if len != Self::LEN && len != Self::LEN - Self::TRAILING_RESERVED {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut buf;
        let bytes = if len == Self::LEN {
            data
        } else {
            // SAFETY: `Self: NoUninit` guarantees the struct has no padding,
            // so `buf` exposes exactly `Self::LEN` bytes of valid storage.
            // The guard above bounds `len == Self::LEN - Self::TRAILING_RESERVED`,
            // so the payload fits in the front of the buffer and the trailing
            // `TRAILING_RESERVED` bytes stay zero — exactly the bit pattern
            // that `Reserved<N>::is_valid_bit_pattern` requires.
            buf = MaybeUninit::<Self>::zeroed();
            unsafe {
                let dst = buf.as_mut_ptr().cast::<u8>();
                core::ptr::copy_nonoverlapping(data.as_ptr(), dst, len);
                core::slice::from_raw_parts(dst, Self::LEN)
            }
        };
        bytemuck::checked::try_pod_read_unaligned::<Self>(bytes)
            .map_err(|_| ProgramError::InvalidInstructionData)
    }
}

/// Marker payload for instructions that take no arguments beyond the
/// dispatcher's discriminator byte.
///
/// Calling [`InstructionArgs::parse`] on `NoArgs` accepts only an empty slice;
/// any non-empty payload returns [`ProgramError::InvalidInstructionData`].
#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, CheckedBitPattern, Zeroable)]
pub struct NoArgs;

impl InstructionArgs for NoArgs {}

#[cfg(test)]
mod tests {
    use bytemuck::{CheckedBitPattern, NoUninit};
    use pinocchio::error::ProgramError;

    use super::{InstructionArgs, NoArgs};
    use crate::wire::Reserved;

    /// Test fixture: `value: u16` (align 2) + `flag: u8` + `_reserved:
    /// Reserved<1>` gives a 4-byte struct with alignment 2 whose final byte
    /// is alignment padding hidden from foreign clients.
    #[repr(C)]
    #[derive(Clone, Copy, NoUninit, CheckedBitPattern, PartialEq, Eq, Debug)]
    struct PaddedArgs {
        value: u16,
        flag: u8,
        _reserved: Reserved<1>,
    }

    impl InstructionArgs for PaddedArgs {
        const TRAILING_RESERVED: usize = 1;
    }

    #[test]
    fn no_args_parses_empty_slice() {
        NoArgs::parse(&[]).expect("empty slice must parse into NoArgs");
    }

    #[test]
    fn no_args_rejects_nonempty_slice() {
        assert_eq!(NoArgs::parse(&[0]).err(), Some(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn no_args_len_is_zero() {
        assert_eq!(NoArgs::LEN, 0);
    }

    #[test]
    fn padded_args_parses_full_length_payload() {
        let args =
            PaddedArgs::parse(&[0x34, 0x12, 0x07, 0x00]).expect("full-length payload parses");
        assert_eq!(args.value, 0x1234);
        assert_eq!(args.flag, 0x07);
    }

    #[test]
    fn padded_args_parses_payload_without_trailing_reserved_byte() {
        let args =
            PaddedArgs::parse(&[0x34, 0x12, 0x07]).expect("short payload parses (zero-filled)");
        assert_eq!(args.value, 0x1234);
        assert_eq!(args.flag, 0x07);
    }

    #[test]
    fn padded_args_rejects_nonzero_reserved_byte() {
        assert_eq!(PaddedArgs::parse(&[0, 0, 0, 1]), Err(ProgramError::InvalidInstructionData),);
    }

    #[test]
    fn padded_args_rejects_payload_shorter_than_tolerance() {
        assert_eq!(PaddedArgs::parse(&[0, 0]), Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn padded_args_rejects_overlong_payload() {
        assert_eq!(PaddedArgs::parse(&[0; 5]), Err(ProgramError::InvalidInstructionData));
    }
}
