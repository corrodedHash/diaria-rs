use crate::{
    derive_key_from_password,
    entry::repository::{DiariaFsRepository, DiariaMetaRepository},
    password::PasswordService,
};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};

use super::version01::SymmetricKey;
use x448::PublicKey as X448PublicKey;

pub struct FsKeyManager<T: DiariaMetaRepository, PW: PasswordService> {
    repo: T,
    password: PW,
}

pub type FsKeyManagerDefault =
    FsKeyManager<DiariaFsRepository, crate::password::TerminalPasswordService>;

impl Default for FsKeyManagerDefault {
    fn default() -> Self {
        Self {
            repo: DiariaFsRepository {},
            password: crate::password::TerminalPasswordService {},
        }
    }
}

impl<T: DiariaMetaRepository, PW: PasswordService> FsKeyManager<T, PW> {
    pub fn new(repo: T, password: PW) -> Self {
        Self { repo, password }
    }
}

#[mockall::automock]
pub trait DiariaKeyManager {
    fn load_private_key(&self) -> [u8; 56];
    fn load_public_key(&self) -> X448PublicKey;
    fn load_symmetric_key(&self) -> SymmetricKey;
}

impl<T: DiariaMetaRepository, PW: PasswordService> DiariaKeyManager for FsKeyManager<T, PW> {
    fn load_symmetric_key(&self) -> SymmetricKey {
        let key_bytes = self.repo.fetch_symmetric_key_raw().unwrap();
        let mut symkey = [0u8; 32];
        symkey.copy_from_slice(&key_bytes[..32]);
        symkey
    }

    fn load_public_key(&self) -> X448PublicKey {
        let key_bytes = self.repo.fetch_public_key_raw().unwrap();
        X448PublicKey::from_bytes(&key_bytes).expect("Invalid public key format")
    }

    fn load_private_key(&self) -> [u8; 56] {
        let key_bytes = self.repo.fetch_private_key_raw().unwrap();
        let cipher_key = CipherPrivateKey::from(key_bytes.as_slice());

        let password = self.password.get_password();
        let encryption_key = derive_key_from_password(&password, &cipher_key.salt);

        let cipher =
            XChaCha20Poly1305::new_from_slice(&encryption_key).expect("Failed to create cipher");

        let decrypted = cipher
            .decrypt(&cipher_key.nonce, cipher_key.ciphertext.as_slice())
            .expect("Failed to decrypt private key");

        let mut private_key = [0u8; 56];
        private_key.copy_from_slice(&decrypted[..56]);
        private_key
    }
}

struct CipherPrivateKey {
    salt: [u8; 32],
    nonce: XNonce,
    ciphertext: Vec<u8>,
}

impl CipherPrivateKey {
    fn serialize(&self) -> Vec<u8> {
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
