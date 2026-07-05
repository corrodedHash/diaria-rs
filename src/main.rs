use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use chrono::Local;
use clap::{Parser, Subcommand};
use dialoguer::Password;
use entry::version01::SymmetricKey;
use std::fs;
use std::path::PathBuf;
use x448::PublicKey as X448PublicKey;
use xdg::BaseDirectories;

mod commands;
mod entry;
mod file_loader;
mod password;
mod stdout_printer;

use commands::*;

#[derive(Parser)]
#[command(version, about, name = "diaria")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Add {
        #[arg(short = 'i', long)]
        input: Option<PathBuf>,
    },
    Read {
        filename: Option<PathBuf>,
    },
    Load {
        #[arg(short = 'd', long)]
        directory: PathBuf,
    },
    Dump {
        #[arg(short = 'd', long)]
        directory: Option<PathBuf>,
    },
    Sync,
    Summarize,
    Stats,
}

fn get_base_dir() -> PathBuf {
    BaseDirectories::with_prefix("diaria")
        .get_data_home()
        .expect("Failed to get base dir")
}

fn get_entries_dir() -> PathBuf {
    let base_dir = get_base_dir();
    let entries_dir = base_dir.join("entries");
    fs::create_dir_all(&entries_dir).expect("Failed to create entries directory");
    entries_dir
}

fn get_entry_path() -> PathBuf {
    let entries_dir = get_entries_dir();
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    entries_dir.join(format!("{}.diaria", timestamp))
}

fn get_password() -> String {
    Password::new()
        .with_prompt("Enter encryption password")
        .interact()
        .expect("Failed to read password")
}

fn derive_key_from_password(password: &str, salt: &SymmetricKey) -> [u8; 32] {
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

fn load_symmetric_key() -> SymmetricKey {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.sym");
    let key_bytes = fs::read(&key_path).expect("Failed to read symmetric key");
    let mut symkey = [0u8; 32];
    symkey.copy_from_slice(&key_bytes[..32]);
    symkey
}

fn load_public_key() -> X448PublicKey {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.pub");
    let key_bytes = fs::read(&key_path).expect("Failed to read public key");
    X448PublicKey::from_bytes(&key_bytes).expect("Invalid public key format")
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

fn load_private_key() -> [u8; 56] {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.key");
    let key_bytes = fs::read(&key_path).expect("Failed to read private key");
    let cipher_key = CipherPrivateKey::from(key_bytes.as_slice());

    let password = get_password();
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => CmdInit::default().execute(),
        Commands::Add { input } => CmdAdd::default().execute(input.as_deref()),
        Commands::Read { filename } => CmdRead::default().execute(filename.as_deref()),
        Commands::Load { directory } => cmd_load(directory),
        Commands::Dump { directory } => cmd_dump(directory),
        Commands::Sync => cmd_sync(),
        Commands::Summarize => cmd_summarize(),
        Commands::Stats => CmdStats::default().execute(),
    }
}
