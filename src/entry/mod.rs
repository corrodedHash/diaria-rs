pub mod key_manager;
pub mod repository;

pub mod version01;

use thiserror::Error;
use version01::SymmetricKey;
use x448::PublicKey as X448PublicKey;
use zeroize::Zeroizing;

/// Every entry file begins with this tag, so an entry is self-identifying even
/// when handled outside a vault.
const MAGIC_TAG: &[u8; 6] = b"DIARIA";

/// Envelope-level errors: the framing around a versioned entry body. Errors
/// from decoding the body itself surface through [`Body`](EntryError::Body).
#[derive(Debug, Error)]
pub enum EntryError {
    #[error("Data too short for magic tag and version")]
    DataTooShort,
    #[error("Invalid magic tag")]
    InvalidMagicTag,
    #[error("Unsupported entry version: {0}")]
    UnsupportedVersion(u8),
    #[error(transparent)]
    Body(#[from] version01::EntryError),
}

/// Encode an entry using the current format version, prepending the envelope
/// header (`MAGIC_TAG || version`) to the versioned body.
pub fn encode(
    long_term_public: &X448PublicKey,
    plaintext: &str,
    salt: &SymmetricKey,
) -> Result<Vec<u8>, EntryError> {
    let body = version01::encode_body(long_term_public, plaintext, salt)?;

    let mut result = Vec::with_capacity(MAGIC_TAG.len() + 1 + body.len());
    result.extend_from_slice(MAGIC_TAG);
    result.push(version01::VERSION);
    result.extend_from_slice(&body);
    Ok(result)
}

/// Decode an entry: validate the magic tag, then dispatch on the version byte
/// to the matching body codec. Adding a `version02` module later is a new
/// `match` arm here — existing entries keep decoding through their own arm.
pub fn decode(
    long_term_private: &[u8; 56],
    data: &[u8],
    salt: &SymmetricKey,
) -> Result<Zeroizing<String>, EntryError> {
    if data.len() < MAGIC_TAG.len() + 1 {
        return Err(EntryError::DataTooShort);
    }

    if &data[..MAGIC_TAG.len()] != MAGIC_TAG {
        return Err(EntryError::InvalidMagicTag);
    }

    let version = data[MAGIC_TAG.len()];
    let body = &data[MAGIC_TAG.len() + 1..];

    match version {
        version01::VERSION => Ok(version01::decode_body(long_term_private, body, salt)?),
        other => Err(EntryError::UnsupportedVersion(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use version01::generate_keypair;

    #[test]
    fn encode_decode_round_trips() {
        let (private_key, public_key) = generate_keypair();
        let salt = [0u8; 32];
        let message = "Hello, this is a secret message!";
        let encoded = encode(&public_key, message, &salt).expect("Encoding failed");
        let decoded = decode(private_key.as_bytes(), &encoded, &salt).expect("Decoding failed");
        assert_eq!(message, decoded.as_str());
    }

    #[test]
    fn encode_stamps_magic_and_version() {
        let (_, public_key) = generate_keypair();
        let salt = [0u8; 32];
        let encoded = encode(&public_key, "hi", &salt).expect("Encoding failed");
        assert_eq!(&encoded[..MAGIC_TAG.len()], MAGIC_TAG);
        assert_eq!(encoded[MAGIC_TAG.len()], version01::VERSION);
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let (private_key, public_key) = generate_keypair();
        let salt = [0u8; 32];
        let mut encoded = encode(&public_key, "hi", &salt).expect("Encoding failed");
        encoded[0] = 0x00;
        let decoded = decode(private_key.as_bytes(), &encoded, &salt);
        std::assert_matches!(decoded, Err(EntryError::InvalidMagicTag));
    }

    #[test]
    fn decode_rejects_unknown_version() {
        let (private_key, public_key) = generate_keypair();
        let salt = [0u8; 32];
        let mut encoded = encode(&public_key, "hi", &salt).expect("Encoding failed");
        encoded[MAGIC_TAG.len()] = 0xFF;
        let decoded = decode(private_key.as_bytes(), &encoded, &salt);
        std::assert_matches!(decoded, Err(EntryError::UnsupportedVersion(0xFF)));
    }
}
