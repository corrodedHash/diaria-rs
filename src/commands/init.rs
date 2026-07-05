use chacha20poly1305::{
    KeyInit as _, XChaCha20Poly1305,
    aead::{Aead as _, Generate as _},
};

use crate::{
    CipherPrivateKey, derive_key_from_password,
    entry::{
        repository::{DiariaFsRepository, DiariaMetaRepository},
        version01::generate_keypair,
    },
    password::{self, PasswordService},
};

fn generate_rand_keys<const T: usize>() -> [u8; T] {
    let mut keys = [0u8; T];
    rand::fill(&mut keys);
    keys
}

pub struct Command<T: DiariaMetaRepository, PW: PasswordService> {
    repo: T,
    password: PW,
}

impl Default for Command<DiariaFsRepository, password::TerminalPasswordService> {
    fn default() -> Self {
        Self {
            repo: DiariaFsRepository {},
            password: password::TerminalPasswordService {},
        }
    }
}

impl<T: DiariaMetaRepository, PW: PasswordService> Command<T, PW> {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.repo.create_structure();

        let password = self.password.get_password();

        let argon2_salt = generate_rand_keys::<32>();
        let symmetric_key = generate_rand_keys::<32>();

        let (private_key, public_key) = generate_keypair();
        let encryption_key = derive_key_from_password(&password, &argon2_salt);

        let cipher =
            XChaCha20Poly1305::new_from_slice(&encryption_key).expect("Failed to create cipher");
        let nonce = chacha20poly1305::XNonce::generate();
        let encrypted_private_key = cipher
            .encrypt(&nonce, &private_key.as_bytes()[..])
            .expect("Failed to encrypt private key");

        let cipher_key = CipherPrivateKey {
            salt: argon2_salt,
            nonce,
            ciphertext: encrypted_private_key,
        };

        self.repo.store_private_key_raw(&cipher_key.serialize())?;
        self.repo.store_public_key_raw(public_key.as_bytes())?;
        self.repo.store_symmetric_key_raw(&symmetric_key)?;

        println!(
            "Initialized diaria in {}",
            self.repo.get_base_dir().display()
        );
        Ok(())
    }
}
