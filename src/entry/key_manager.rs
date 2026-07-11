use crate::{
    crypto::{CipherPrivateKey, derive_key_from_password},
    entry::repository::DiariaMetaRepository,
    util::password::PasswordService,
};
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, KeyInit},
};
use zeroize::Zeroizing;

use super::version01::SymmetricKey;
use crate::manifest::{Manifest, ManifestError};
use thiserror::Error;
use x448::PublicKey as X448PublicKey;

/// Errors surfaced while loading key material from the vault.
#[derive(Debug, Error)]
pub enum KeyError {
    /// The private key could not be decrypted — almost always a wrong password.
    #[error("failed to decrypt the private key (wrong password?)")]
    Decryption,
    /// A key file could not be read from the vault.
    #[error("failed to read key material from the vault")]
    Io,
    /// Key data has an unexpected format or length.
    #[error("invalid key data")]
    InvalidKeyData,
    /// The AEAD cipher could not be initialised (should not happen).
    #[error("failed to initialise cipher")]
    CipherInit,
}

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
    fn load_private_key(&self) -> Result<Zeroizing<[u8; 56]>, KeyError>;
    fn load_public_key(&self) -> Result<X448PublicKey, KeyError>;
    fn load_symmetric_key(&self) -> Result<SymmetricKey, KeyError>;
    /// Read and validate the vault's format version from its manifest.
    ///
    /// Fails with [`ManifestError::LegacyUnversioned`] for a pre-versioning
    /// vault and [`ManifestError::UnknownVersion`] for one written by a newer
    /// binary, so callers can refuse to operate on a format they don't fully
    /// understand.
    fn load_manifest_version(&self) -> Result<u32, ManifestError>;
}

impl DiariaKeyManager for FsKeyManager {
    fn load_symmetric_key(&self) -> Result<SymmetricKey, KeyError> {
        let key_bytes = self
            .repo
            .fetch_symmetric_key_raw()
            .map_err(|_| KeyError::Io)?;
        let mut symkey = [0u8; 32];
        let slice = key_bytes.get(..32).ok_or(KeyError::InvalidKeyData)?;
        symkey.copy_from_slice(slice);
        Ok(symkey)
    }

    fn load_public_key(&self) -> Result<X448PublicKey, KeyError> {
        let key_bytes = self.repo.fetch_public_key_raw().map_err(|_| KeyError::Io)?;
        X448PublicKey::from_bytes(&key_bytes).ok_or(KeyError::InvalidKeyData)
    }

    fn load_manifest_version(&self) -> Result<u32, ManifestError> {
        let raw = self
            .repo
            .fetch_manifest_raw()
            .map_err(|_| ManifestError::Malformed)?
            .ok_or(ManifestError::LegacyUnversioned)?;
        Manifest::parse(&raw).map(|m| m.version)
    }

    fn load_private_key(&self) -> Result<Zeroizing<[u8; 56]>, KeyError> {
        let key_bytes = self
            .repo
            .fetch_private_key_raw()
            .map_err(|_| KeyError::Io)?;
        let cipher_key = CipherPrivateKey::from(key_bytes.as_slice());

        let password = self.password.get_password();
        let encryption_key = derive_key_from_password(&password, &cipher_key.salt);

        let cipher = XChaCha20Poly1305::new_from_slice(&*encryption_key)
            .map_err(|_| KeyError::CipherInit)?;

        let decrypted = Zeroizing::new(
            cipher
                .decrypt(&cipher_key.nonce, cipher_key.ciphertext.as_slice())
                .map_err(|_| KeyError::Decryption)?,
        );

        let mut private_key = Zeroizing::new([0u8; 56]);
        let slice = decrypted.get(..56).ok_or(KeyError::InvalidKeyData)?;
        private_key.copy_from_slice(slice);
        Ok(private_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::repository::MockDiariaMetaRepository;
    use crate::util::password::MockPasswordService;

    /// Helper: build an `FsKeyManager` wired to a mock repo. The password
    /// service is irrelevant for `load_symmetric_key` and `load_public_key` since
    /// they never prompt for a password.
    fn make_key_manager(repo: MockDiariaMetaRepository) -> FsKeyManager {
        FsKeyManager::new(Box::new(repo), Box::new(MockPasswordService::new()))
    }

    #[test]
    fn load_symmetric_key_returns_io_error_on_fetch_error() {
        let mut repo = MockDiariaMetaRepository::new();
        repo.expect_fetch_symmetric_key_raw().returning(|| {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )))
        });

        let km = make_key_manager(repo);
        assert!(matches!(km.load_symmetric_key(), Err(KeyError::Io)));
    }

    #[test]
    fn load_public_key_returns_io_error_on_fetch_error() {
        let mut repo = MockDiariaMetaRepository::new();
        repo.expect_fetch_public_key_raw().returning(|| {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )))
        });

        let km = make_key_manager(repo);
        assert!(matches!(km.load_public_key(), Err(KeyError::Io)));
    }

    #[test]
    fn load_public_key_returns_invalid_key_data_on_bad_bytes() {
        let mut repo = MockDiariaMetaRepository::new();
        repo.expect_fetch_public_key_raw()
            .returning(|| Ok(vec![0u8; 10]));

        let km = make_key_manager(repo);
        assert!(matches!(
            km.load_public_key(),
            Err(KeyError::InvalidKeyData)
        ));
    }

    #[test]
    fn load_symmetric_key_returns_invalid_key_data_on_short_file() {
        let mut repo = MockDiariaMetaRepository::new();
        repo.expect_fetch_symmetric_key_raw()
            .returning(|| Ok(vec![0u8; 16]));

        let km = make_key_manager(repo);
        assert!(matches!(
            km.load_symmetric_key(),
            Err(KeyError::InvalidKeyData)
        ));
    }
}
