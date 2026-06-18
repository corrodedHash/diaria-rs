use chrono::Local;
use clap::{Parser, Subcommand};
use entry::version01::SymmetricKey;
use std::fs;
use std::path::PathBuf;
use x448::{EphemeralSecret as X448PrivateKey, PublicKey as X448PublicKey};
use xdg::BaseDirectories;

mod entry;

#[derive(Parser)]
#[command(name = "diaria")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add,
    Read,
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

fn load_symmetric_key() -> SymmetricKey {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.sym");
    let key_bytes = fs::read(&key_path).expect("Failed to read symmetric key");
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&key_bytes[..32]);
    salt
}

fn load_public_key() -> X448PublicKey {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.pub");
    let key_bytes = fs::read(&key_path).expect("Failed to read public key");
    X448PublicKey::from_bytes(&key_bytes).expect("Invalid public key format")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let salt = load_symmetric_key();
    let public_key = load_public_key();

    match cli.command {
        Commands::Add => {
            let entry_path = get_entry_path();
            let encoded = entry::version01::encode(&public_key, "", &salt)?;
            fs::write(&entry_path, encoded)?;
            println!("Created entry: {}", entry_path.display());
        }
        Commands::Read => {
            let entries_dir = get_entries_dir();
            for entry in fs::read_dir(entries_dir)? {
                let entry = entry?;
                println!("{}", entry.path().display());
            }
        }
    }
    Ok(())
}
