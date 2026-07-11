use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use chacha20poly1305::XNonce;
use zeroize::Zeroizing;

/// Derives a 32-byte symmetric key from a password and salt using Argon2.
///
/// # Panics
///
/// Panics if the underlying Argon2 library encounters an error (in practice,
/// in-memory hashing with valid parameters is infallible).
#[allow(clippy::expect_used)]
pub fn derive_key_from_password(password: &str, salt: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let argon2 = Argon2::default();
    let salt_string = SaltString::encode_b64(salt).expect("Failed to encode salt");

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .expect("Failed to hash password");

    // Use the raw Argon2 hash *output* — the actual derived key material, which
    // depends on the password. (Reading bytes off the PHC *string* instead would
    // capture only its parameter/salt prefix, making the key independent of the
    // password.) The default Argon2 output is exactly 32 bytes.
    let hash = password_hash
        .hash
        .expect("Argon2 password hash always carries an output");
    let mut key = Zeroizing::new([0u8; 32]);
    key.copy_from_slice(hash.as_bytes());
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
