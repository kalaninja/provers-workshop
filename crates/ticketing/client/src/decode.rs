//! Discriminator/owner-checked account decoders (hand-written; not codama
//! output).
//!
//! The generated `Account::from_bytes` / `TryFrom<&AccountInfo>` decoders
//! borsh-deserialize from offset 0 without validating the 8-byte discriminator
//! or the account owner (the `@codama/renderers-rust` `from_bytes` template is
//! a fixed `Self::deserialize(..)`, with no option to add a check — see the
//! crate-level caveats). These helpers add that gate for callers that decode an
//! account they did not derive themselves: an address from user input, an
//! indexer sweep, or a stored pointer field such as `ticket.event`.

use borsh::BorshDeserialize;

use crate::accounts::{EVENT_DISCRIMINATOR, Event, TICKET_DISCRIMINATOR, Ticket};

/// A ticketing-program account whose first 8 bytes are a fixed discriminator.
///
/// Implemented for every generated account so [`decode_checked`] can verify the
/// discriminator before borsh-deserializing. The provided methods give a
/// method-call form of the free functions, e.g. `Event::from_bytes_checked`
/// (bring [`CheckedAccount`] into scope to call them).
pub trait CheckedAccount: BorshDeserialize {
    /// The account's canonical 8-byte discriminator (its `header` value).
    const DISCRIMINATOR: [u8; 8];

    /// Borsh-decodes `data` as `Self`, verifying the discriminator first — the
    /// method form of [`decode_checked`].
    ///
    /// # Errors
    ///
    /// See [`decode_checked`].
    fn from_bytes_checked(data: &[u8]) -> Result<Self, DecodeError> {
        decode_checked::<Self>(data)
    }

    /// Borsh-decodes a fetched [`solana_account::Account`] as `Self`, verifying
    /// both owner and discriminator — the method form of
    /// [`decode_checked_account`].
    ///
    /// # Errors
    ///
    /// See [`decode_checked_account`].
    #[cfg(feature = "fetch")]
    fn from_account_checked(account: &solana_account::Account) -> Result<Self, DecodeError> {
        decode_checked_account::<Self>(account)
    }
}

macro_rules! impl_checked_account {
    ($ty:ty, $discriminator:expr) => {
        impl CheckedAccount for $ty {
            const DISCRIMINATOR: [u8; 8] = $discriminator;
        }
    };
}

impl_checked_account!(Event, EVENT_DISCRIMINATOR);
impl_checked_account!(Ticket, TICKET_DISCRIMINATOR);

/// Reasons a checked decode can fail.
#[derive(Debug)]
pub enum DecodeError {
    /// Fewer than 8 bytes — no room for a discriminator.
    TooShort,
    /// The leading 8 bytes did not match the target account's discriminator.
    DiscriminatorMismatch,
    /// The account is not owned by the ticketing program.
    WrongOwner,
    /// The bytes passed the discriminator gate but borsh deserialization
    /// failed.
    Deserialize(std::io::Error),
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort => f.write_str("account data shorter than an 8-byte discriminator"),
            Self::DiscriminatorMismatch => {
                f.write_str("account discriminator does not match the requested type")
            },
            Self::WrongOwner => f.write_str("account is not owned by the ticketing program"),
            Self::Deserialize(err) => write!(f, "borsh deserialization failed: {err}"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Deserialize(err) => Some(err),
            _ => None,
        }
    }
}

/// Borsh-decodes `data` as `T`, but only if its leading 8 bytes equal
/// `T::DISCRIMINATOR` — guarding against decoding a wrong-type or
/// uninitialized account into a garbage value (both ticketing accounts share
/// the same 16-byte header/`created_slot` prefix, so the discriminator is the
/// only reliable type tag).
///
/// The owner is not visible from raw bytes; use [`decode_checked_account`] (or
/// check the owner yourself) when decoding an account you did not derive.
///
/// # Errors
///
/// Returns [`DecodeError::TooShort`] if `data` is under 8 bytes,
/// [`DecodeError::DiscriminatorMismatch`] if the header is wrong, or
/// [`DecodeError::Deserialize`] if borsh fails on otherwise-gated bytes —
/// including trailing bytes beyond the account layout, which
/// `try_from_slice` rejects.
pub fn decode_checked<T: CheckedAccount>(data: &[u8]) -> Result<T, DecodeError> {
    let header = data.get(..8).ok_or(DecodeError::TooShort)?;
    if header != T::DISCRIMINATOR {
        return Err(DecodeError::DiscriminatorMismatch);
    }
    T::try_from_slice(data).map_err(DecodeError::Deserialize)
}

/// Borsh-decodes a fetched [`solana_account::Account`] as `T`, verifying both
/// that the account is owned by [`TICKETING_ID`](crate::TICKETING_ID) and that
/// its discriminator matches before deserializing. Prefer this over
/// [`decode_checked`] for accounts returned by RPC, where the owner check
/// rules out a look-alike account sitting at an attacker-chosen address.
///
/// # Errors
///
/// Returns [`DecodeError::WrongOwner`] if the account is owned by another
/// program, otherwise any error from [`decode_checked`].
#[cfg(feature = "fetch")]
pub fn decode_checked_account<T: CheckedAccount>(
    account: &solana_account::Account,
) -> Result<T, DecodeError> {
    if account.owner.to_bytes() != crate::TICKETING_ID.to_bytes() {
        return Err(DecodeError::WrongOwner);
    }
    decode_checked(&account.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid `Event` account body: its discriminator followed by
    /// zeroes to its 152-byte length (zeroed `EventStatus` is `SalesOpen`, so
    /// zeroes deserialize).
    fn event_bytes() -> Vec<u8> {
        let mut data = EVENT_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&[0u8; 144]);
        data
    }

    #[test]
    fn decodes_when_discriminator_matches() {
        let event = decode_checked::<Event>(&event_bytes()).expect("event decodes");
        assert_eq!(event.header, EVENT_DISCRIMINATOR);
    }

    #[test]
    fn rejects_wrong_type_before_deserializing() {
        // Event bytes (152 B) would mis-decode as the 112-byte Ticket, but the
        // discriminator gate rejects them first, without touching borsh.
        assert!(matches!(
            decode_checked::<Ticket>(&event_bytes()),
            Err(DecodeError::DiscriminatorMismatch),
        ));
    }

    #[test]
    fn rejects_data_too_short_for_a_discriminator() {
        assert!(matches!(decode_checked::<Event>(&[0u8; 4]), Err(DecodeError::TooShort)));
    }

    #[test]
    fn rejects_trailing_bytes_after_the_account_layout() {
        let mut data = event_bytes();
        data.push(0);

        assert!(matches!(decode_checked::<Event>(&data), Err(DecodeError::Deserialize(_))));
    }

    #[test]
    fn method_form_matches_the_free_function() {
        let event = Event::from_bytes_checked(&event_bytes()).expect("event decodes via method");
        assert_eq!(event.header, EVENT_DISCRIMINATOR);
    }
}
