use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Generate, KeyInit},
};
use std::io::Read;
use thiserror::Error;
use x448::{EphemeralSecret as X448PrivateKey, PublicKey as X448PublicKey, x448};
use zeroize::Zeroizing;

pub type SymmetricKey = [u8; 32];

/// The version byte this module reads and writes. The envelope framing (magic
/// tag + version dispatch) lives in [`super`]; this module owns only the v1
/// body codec.
pub const VERSION: u8 = 1;

/// Fixed HKDF salt for domain separation. The per-vault symmetric key is mixed
/// into the HKDF input key material (see [`derive_aead_key`]) so it acts as a
/// local secret on top of X448, not as a public salt.
const HKDF_SALT: &[u8] = b"diaria-v1-entry";

/// HKDF info string binding the derived key to its single use.
const HKDF_INFO: &[u8] = b"diaria entry AEAD key";

#[derive(Debug, Error)]
pub enum EntryError {
    #[error("Ciphertext too short")]
    CiphertextTooShort,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Decryption failed")]
    DecryptionFailed(chacha20poly1305::Error),
    #[error("Chacha20Poly1305 error: {0}")]
    Chacha20Poly1305(#[from] chacha20poly1305::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Chacha20Poly1305 key length mismatch: {0}")]
    ChachaKeylengthMismatch(#[from] sha2::digest::InvalidLength),
}

pub fn generate_keypair() -> (X448PrivateKey, X448PublicKey) {
    let private_key = X448PrivateKey::generate();
    let public_key = X448PublicKey::from(&private_key);
    (private_key, public_key)
}

/// Derive the per-entry AEAD key: HKDF-SHA256 over the X448 shared secret
/// concatenated with the per-vault symmetric key. The symmetric key is secret
/// (it never leaves the local vault dir), so it is input key material here —
/// not a public salt. Breaking X448 alone is not enough to recover this key.
fn derive_aead_key(shared_secret: &[u8], symmetric_key: &SymmetricKey) -> chacha20poly1305::Key {
    let mut ikm = Vec::with_capacity(shared_secret.len() + symmetric_key.len());
    ikm.extend_from_slice(shared_secret);
    ikm.extend_from_slice(symmetric_key);

    let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(HKDF_SALT), &ikm);
    let mut okm = chacha20poly1305::Key::default();
    hk.expand(HKDF_INFO, &mut okm)
        .expect("32 is a valid length for Sha256 to output");

    okm
}

fn derive_shared_secret_initial(
    private_key: &X448PrivateKey,
    peer_public_key: &X448PublicKey,
    symmetric_key: &SymmetricKey,
) -> chacha20poly1305::Key {
    let shared = private_key.diffie_hellman(peer_public_key);
    derive_aead_key(shared.as_bytes(), symmetric_key)
}

fn derive_shared_secret_later(
    private_key: &[u8; 56],
    peer_public_key: &X448PublicKey,
    symmetric_key: &SymmetricKey,
) -> chacha20poly1305::Key {
    let shared =
        x448(*private_key, *peer_public_key.as_bytes()).expect("Failed to compute shared secret");
    derive_aead_key(&shared, symmetric_key)
}

fn encrypt(
    long_term_public: &X448PublicKey,
    plaintext: &[u8],
    symmetric_key: &SymmetricKey,
) -> Result<Vec<u8>, EntryError> {
    let (ephemeral_private, ephemeral_public) = generate_keypair();
    let shared_secret =
        derive_shared_secret_initial(&ephemeral_private, long_term_public, symmetric_key);

    let cipher = XChaCha20Poly1305::new(&shared_secret);
    let nonce = chacha20poly1305::XNonce::generate();
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(EntryError::Chacha20Poly1305)?;

    let mut result = Vec::with_capacity(56 + 24 + ciphertext.len());
    result.extend_from_slice(ephemeral_public.as_bytes());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

fn decrypt(
    long_term_private: &[u8; 56],
    ciphertext: &[u8],
    symmetric_key: &SymmetricKey,
) -> Result<Zeroizing<Vec<u8>>, EntryError> {
    if ciphertext.len() < 80 {
        return Err(EntryError::CiphertextTooShort);
    }

    let ephemeral_public =
        X448PublicKey::from_bytes(&ciphertext[0..56]).ok_or(EntryError::InvalidPublicKey)?;
    let nonce = XNonce::try_from(&ciphertext[56..80]).expect("This should always work");
    let actual_ciphertext = &ciphertext[80..];

    let shared_secret =
        derive_shared_secret_later(long_term_private, &ephemeral_public, symmetric_key);

    let cipher = XChaCha20Poly1305::new_from_slice(&shared_secret)
        .map_err(EntryError::ChachaKeylengthMismatch)?;

    let plaintext = Zeroizing::new(
        cipher
            .decrypt(&nonce, actual_ciphertext)
            .map_err(EntryError::DecryptionFailed)?,
    );

    Ok(plaintext)
}

fn compress(input: &[u8]) -> Result<Zeroizing<Vec<u8>>, EntryError> {
    let compress_reader = brotli::CompressorReader::new(input, 4096, 11, 22);
    let buf: Vec<u8> = std::io::BufReader::new(compress_reader)
        .bytes()
        .collect::<Result<Vec<u8>, _>>()
        .map_err(EntryError::Io)?;
    Ok(Zeroizing::new(buf))
}

fn decompress(input: &[u8]) -> Result<Zeroizing<Vec<u8>>, EntryError> {
    let mut input = brotli::Decompressor::new(input, 4096);
    let mut buf = Vec::new();
    input.read_to_end(&mut buf).map_err(EntryError::Io)?;
    Ok(Zeroizing::new(buf))
}

/// Encode an entry body (no envelope header — the caller in [`super`] prepends
/// the magic tag and version byte).
pub fn encode_body(
    long_term_public: &X448PublicKey,
    plaintext: &str,
    symmetric_key: &SymmetricKey,
) -> Result<Vec<u8>, EntryError> {
    let compressed = compress(plaintext.as_bytes())?;
    encrypt(long_term_public, &compressed, symmetric_key)
}

/// Decode an entry body (the envelope header has already been stripped and
/// validated by [`super`]).
pub fn decode_body(
    long_term_private: &[u8; 56],
    body: &[u8],
    symmetric_key: &SymmetricKey,
) -> Result<Zeroizing<String>, EntryError> {
    let plaintext = decrypt(long_term_private, body, symmetric_key)?;
    let mut decompressed = decompress(&plaintext)?;
    let raw = std::mem::take(&mut *decompressed);
    Ok(Zeroizing::from(
        String::from_utf8(raw).map_err(EntryError::Utf8)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let (long_term_private, long_term_public) = generate_keypair();
        let message = Vec::from(b"Hello, this is a secret message!");
        let symmetric_key = [0u8; 32];
        let encrypted =
            encrypt(&long_term_public, &message, &symmetric_key).expect("Encryption failed");
        let decrypted = decrypt(long_term_private.as_bytes(), &encrypted, &symmetric_key)
            .expect("Decryption failed");
        assert_eq!(message.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_compress_decompress() {
        let message = Vec::from(b"Hello, this is a secret message!");
        let compressed = compress(&message).expect("Compression failed");
        let decompressed = decompress(&compressed).expect("Decompression failed");
        assert_eq!(message.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_encode_decode_body() {
        let (long_term_private, long_term_public) = generate_keypair();
        let message = "Hello, this is a secret message!";
        let symmetric_key = [0u8; 32];
        let encoded =
            encode_body(&long_term_public, message, &symmetric_key).expect("Encoding failed");
        let decoded = decode_body(long_term_private.as_bytes(), &encoded, &symmetric_key)
            .expect("Decoding failed");
        assert_eq!(message, decoded.as_str());
    }
}
