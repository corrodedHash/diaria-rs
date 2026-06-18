use chrono::Local;
use clap::{Parser, Subcommand};
use dialoguer::{Editor, FuzzySelect};
use entry::version01::{SymmetricKey, decode};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use x448::PublicKey as X448PublicKey;
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

fn load_private_key() -> [u8; 56] {
    let base_dir = get_base_dir();
    let key_path = base_dir.join("key.priv");
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
            todo!()
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
                        .items(
                            &entries
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect::<Vec<_>>(),
                        )
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
