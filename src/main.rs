use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, Generate as _, KeyInit},
};
use chrono::Local;
use clap::{Parser, Subcommand};
use dialoguer::{Editor, FuzzySelect, Password};
use entry::version01::{SymmetricKey, decode, generate_keypair};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use x448::PublicKey as X448PublicKey;
use xdg::BaseDirectories;

mod entry;

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

fn create_base_dir() -> PathBuf {
    let base_dir = get_base_dir();
    fs::create_dir_all(&base_dir).expect("Failed to create base directory");
    base_dir
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

fn load_private_key() -> [u8; 56] {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.key");
    let key_bytes = fs::read(&key_path).expect("Failed to read private key");
    let mut private_key = [0u8; 56];
    private_key.copy_from_slice(&key_bytes[..56]);
    private_key
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let salt = load_symmetric_key();
    let public_key = load_public_key();

    match cli.command {
        Commands::Init => {
            let base_dir = create_base_dir();
            let entries_dir = base_dir.join("entries");
            fs::create_dir_all(&entries_dir).expect("Failed to create entries directory");

            let password = get_password();
            let argon2_salt = {
                let mut salt = [0u8; 32];
                rand::fill(&mut salt);
                salt
            };

            let (private_key, public_key) = generate_keypair();
            let symmetric_key = argon2_salt;
            let encryption_key = derive_key_from_password(&password, &argon2_salt);

            let cipher = XChaCha20Poly1305::new_from_slice(&encryption_key)
                .expect("Failed to create cipher");
            let nonce = chacha20poly1305::XNonce::generate();
            let encrypted_private_key = cipher
                .encrypt(&nonce, &private_key.as_bytes()[..])
                .expect("Failed to encrypt private key");

            let mut key_file = Vec::with_capacity(24 + encrypted_private_key.len());
            key_file.extend_from_slice(&nonce);
            key_file.extend_from_slice(&encrypted_private_key);

            fs::write(base_dir.join("key.key"), &key_file)?;
            fs::write(base_dir.join("key.pub"), public_key.as_bytes())?;
            fs::write(base_dir.join("key.sym"), symmetric_key)?;

            println!("Initialized diaria in {}", base_dir.display());
        }
        Commands::Add { input } => {
            let entry_path = get_entry_path();
            let input = if let Some(p) = input {
                fs::read_to_string(p)?
            } else {
                Editor::new().edit("")?.unwrap_or_default()
            };
            let encoded = entry::version01::encode(&public_key, &input, &salt)?;
            fs::write(&entry_path, encoded)?;
            println!("Created entry: {}", entry_path.display());
        }
        Commands::Read { filename } => {
            let entries_dir = get_entries_dir();
            let entry_path = if let Some(f) = filename {
                f
            } else {
                let entries = fs::read_dir(&entries_dir)?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect::<Vec<_>>();

                if entries.is_empty() {
                    println!("No entries found in {}", entries_dir.display());
                    return Ok(());
                }

                let selection =
                    FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Select an entry")
                        .items(entries.iter().map(|p| p.display()).collect::<Vec<_>>())
                        .interact()?;

                entries[selection].clone()
            };

            let private_key = load_private_key();
            let data = fs::read(&entry_path)?;
            let plaintext = decode(&private_key, &data, &salt)?;
            println!("{}", plaintext);
        }
        Commands::Load { directory } => {
            let entries_dir = get_entries_dir();
            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let content = fs::read_to_string(&path)?;
                    let encoded = entry::version01::encode(&public_key, &content, &salt)?;
                    let dest_path = entries_dir.join(path.file_name().unwrap());
                    fs::write(dest_path, encoded)?;
                }
            }
            println!("Loaded entries from {}", directory.display());
        }
        Commands::Dump { directory } => {
            let entries_dir = get_entries_dir();
            let output_dir = directory.unwrap_or_else(|| PathBuf::from("./dump"));
            fs::create_dir_all(&output_dir)?;

            let private_key = load_private_key();
            for entry in fs::read_dir(&entries_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "diaria") {
                    let data = fs::read(&path)?;
                    let plaintext = decode(&private_key, &data, &salt)?;
                    let dest_path = output_dir.join(path.file_stem().unwrap());
                    fs::write(dest_path, plaintext)?;
                }
            }
            println!("Dumped entries to {}", output_dir.display());
        }
        Commands::Sync => {
            let entries_dir = get_entries_dir();

            if !entries_dir.join(".git").exists() {
                println!("Not a git repository: {}", entries_dir.display());
                return Ok(());
            }

            Command::new("git")
                .arg("-C")
                .arg(&entries_dir)
                .arg("add")
                .arg("*.diaria")
                .status()?;

            Command::new("git")
                .arg("-C")
                .arg(&entries_dir)
                .arg("commit")
                .arg("-m")
                .arg("Auto-commit entries")
                .status()?;

            Command::new("git")
                .arg("-C")
                .arg(&entries_dir)
                .arg("push")
                .status()?;

            Command::new("git")
                .arg("-C")
                .arg(&entries_dir)
                .arg("pull")
                .status()?;

            println!("Synced entries repository");
        }
        Commands::Summarize => {
            let entries_dir = get_entries_dir();
            let private_key = load_private_key();
            let now = Local::now();

            let time_offsets = [
                now - chrono::Duration::days(1),
                now - chrono::Duration::days(7),
                now - chrono::Duration::days(30),
                now - chrono::Duration::days(365),
                now - chrono::Duration::days(730),
                now - chrono::Duration::days(1460),
                now - chrono::Duration::days(2920),
            ];

            for offset in time_offsets {
                let date_str = offset.format("%Y-%m-%d").to_string();

                for entry in fs::read_dir(&entries_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if let Some(name) = path.file_name()
                        && let Some(name_str) = name.to_str()
                        && name_str.contains(&date_str)
                    {
                        let data = fs::read(&path)?;
                        let plaintext = decode(&private_key, &data, &salt)?;
                        println!("=== Entry from {} ===", date_str);
                        println!("{}", plaintext);
                        println!();
                    }
                }
            }
        }
        Commands::Stats => {
            todo!()
        }
    }
    Ok(())
}
