use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use chacha20poly1305::XNonce;

/// Derives a 32-byte symmetric key from a password and salt using Argon2.
pub fn derive_key_from_password(password: &str, salt: &[u8; 32]) -> [u8; 32] {
    let argon2 = Argon2::default();
    let salt_string = SaltString::encode_b64(salt).expect("Failed to encode salt");

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .expect("Failed to hash password");

    let hash_string = password_hash.to_string();
    let hash_bytes = hash_string.as_bytes();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes[..32]);
    key
}

/// The password-encrypted private key as stored on disk: `salt || nonce || ciphertext`.
pub struct CipherPrivateKey {
    pub salt: [u8; 32],
    pub nonce: XNonce,
    pub ciphertext: Vec<u8>,
}

impl CipherPrivateKey {
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(32 + 24 + self.ciphertext.len());
        result.extend_from_slice(&self.salt);
        result.extend_from_slice(&self.nonce);
        result.extend_from_slice(&self.ciphertext);
        result
    }
}

impl From<&[u8]> for CipherPrivateKey {
    fn from(data: &[u8]) -> Self {
        let (salt, rest) = data.split_at(32);
        let (nonce, ciphertext) = rest.split_at(24);
        let mut salt_array = [0u8; 32];
        salt_array.copy_from_slice(salt);
        let mut nonce_array = [0u8; 24];
        nonce_array.copy_from_slice(nonce);
        Self {
            salt: salt_array,
            nonce: XNonce::from(nonce_array),
            ciphertext: ciphertext.to_vec(),
        }
    }
}
