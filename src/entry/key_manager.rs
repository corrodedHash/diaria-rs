use crate::{
    crypto::{CipherPrivateKey, derive_key_from_password},
    entry::repository::DiariaMetaRepository,
    password::PasswordService,
};
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, KeyInit},
};

use super::version01::SymmetricKey;
use crate::manifest::{Manifest, ManifestError};
use x448::PublicKey as X448PublicKey;

pub struct FsKeyManager {
    repo: Box<dyn DiariaMetaRepository>,
    password: Box<dyn PasswordService>,
}

impl FsKeyManager {
    pub fn new(repo: Box<dyn DiariaMetaRepository>, password: Box<dyn PasswordService>) -> Self {
        Self { repo, password }
    }
}

#[mockall::automock]
pub trait DiariaKeyManager {
    fn load_private_key(&self) -> [u8; 56];
    fn load_public_key(&self) -> X448PublicKey;
    fn load_symmetric_key(&self) -> SymmetricKey;
    /// Read and validate the vault's format version from its manifest.
    ///
    /// Fails with [`ManifestError::LegacyUnversioned`] for a pre-versioning
    /// vault and [`ManifestError::UnknownVersion`] for one written by a newer
    /// binary, so callers can refuse to operate on a format they don't fully
    /// understand.
    fn load_manifest_version(&self) -> Result<u32, ManifestError>;
}

impl DiariaKeyManager for FsKeyManager {
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

    fn load_manifest_version(&self) -> Result<u32, ManifestError> {
        let raw = self
            .repo
            .fetch_manifest_raw()
            .map_err(|_| ManifestError::Malformed)?
            .ok_or(ManifestError::LegacyUnversioned)?;
        Manifest::parse(&raw).map(|m| m.version)
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
