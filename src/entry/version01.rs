use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Generate, KeyInit},
};
use thiserror::Error;
use x448::{EphemeralSecret as X448PrivateKey, PublicKey as X448PublicKey};

const MAGIC_TAG: &[u8; 6] = b"DIARIA";
const VERSION: u8 = 1;

#[derive(Debug, Error, PartialEq)]
pub enum EntryError {
    #[error("Ciphertext too short")]
    CiphertextTooShort,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Invalid nonce")]
    InvalidNonce,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid UTF-8")]
    InvalidUtf8,
    #[error("Data too short for magic tag and version")]
    DataTooShort,
    #[error("Invalid magic tag")]
    InvalidMagicTag,
    #[error("Unsupported version")]
    UnsupportedVersion,
}

fn generate_keypair() -> (X448PrivateKey, X448PublicKey) {
    let private_key = X448PrivateKey::generate();
    let public_key = X448PublicKey::from(&private_key);
    (private_key, public_key)
}

fn derive_shared_secret(
    private_key: &X448PrivateKey,
    peer_public_key: &X448PublicKey,
) -> chacha20poly1305::Key {
    let ikm = private_key.diffie_hellman(peer_public_key);

    let hk = hkdf::Hkdf::<sha2::Sha256>::new(None, ikm.as_bytes());
    let mut okm = chacha20poly1305::Key::default();
    hk.expand(b"diaria entry shared secret", &mut okm)
        .expect("42 is a valid length for Sha256 to output");

    okm
}

fn encrypt(long_term_public: &X448PublicKey, plaintext: &str) -> Result<Vec<u8>, EntryError> {
    let (ephemeral_private, ephemeral_public) = generate_keypair();
    let shared_secret = derive_shared_secret(&ephemeral_private, long_term_public);

    let cipher = XChaCha20Poly1305::new(&shared_secret);
    let nonce = chacha20poly1305::XNonce::generate();
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| EntryError::EncryptionFailed)?;

    let mut result = Vec::with_capacity(56 + 24 + ciphertext.len());
    result.extend_from_slice(ephemeral_public.as_bytes());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

fn decrypt(long_term_private: &X448PrivateKey, ciphertext: &[u8]) -> Result<String, EntryError> {
    if ciphertext.len() < 80 {
        return Err(EntryError::CiphertextTooShort);
    }

    let ephemeral_public =
        X448PublicKey::from_bytes(&ciphertext[0..56]).ok_or(EntryError::InvalidPublicKey)?;
    let nonce = XNonce::try_from(&ciphertext[56..80]).map_err(|_| EntryError::InvalidNonce)?;
    let actual_ciphertext = &ciphertext[80..];

    let shared_secret = derive_shared_secret(long_term_private, &ephemeral_public);
    let cipher = XChaCha20Poly1305::new_from_slice(&shared_secret)
        .map_err(|_| EntryError::DecryptionFailed)?;

    let plaintext = cipher
        .decrypt(&nonce, actual_ciphertext)
        .map_err(|_| EntryError::DecryptionFailed)?;
    Ok(String::from_utf8(plaintext).map_err(|_| EntryError::InvalidUtf8)?)
}

pub fn encode(long_term_public: &X448PublicKey, plaintext: &str) -> Result<Vec<u8>, EntryError> {
    let encrypted = encrypt(long_term_public, plaintext)?;

    let mut result = Vec::with_capacity(6 + 1 + encrypted.len());
    result.extend_from_slice(MAGIC_TAG);
    result.push(VERSION);
    result.extend_from_slice(&encrypted);

    Ok(result)
}

pub fn decode(
    long_term_private: &crate::X448PrivateKey,
    data: &[u8],
) -> Result<String, EntryError> {
    if data.len() < 7 {
        return Err(EntryError::DataTooShort);
    }

    if &data[0..6] != MAGIC_TAG {
        return Err(EntryError::InvalidMagicTag);
    }

    if data[6] != VERSION {
        return Err(EntryError::UnsupportedVersion);
    }

    let ciphertext = &data[7..];
    decrypt(long_term_private, ciphertext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let (long_term_private, long_term_public) = generate_keypair();
        let message = "Hello, this is a secret message!";
        let encrypted = encrypt(&long_term_public, message).expect("Encryption failed");
        let decrypted = decrypt(&long_term_private, &encrypted).expect("Decryption failed");
        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_encode_decode() {
        let (long_term_private, long_term_public) = generate_keypair();
        let message = "Hello, this is a secret message!";
        let encoded = encode(&long_term_public, message).expect("Encoding failed");
        let decoded = decode(&long_term_private, &encoded).expect("Decoding failed");
        assert_eq!(message, decoded);
    }

    #[test]
    fn test_decode_fails() {
        let (long_term_private, long_term_public) = generate_keypair();
        let message = "Hello, this is a secret message!";
        let mut encoded = encode(&long_term_public, message).expect("Encoding failed");
        encoded[0] = 0x00; // Corrupt the magic tag
        let decoded = decode(&long_term_private, &encoded);
        assert_eq!(decoded, Err(EntryError::InvalidMagicTag));
    }
}
