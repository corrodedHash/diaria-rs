use crate::{
    CipherPrivateKey,
    entry::version01::{decode, encode, generate_keypair},
};
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, Generate as _, KeyInit},
};
use chrono::Local;
use dialoguer::{Editor, FuzzySelect};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::{
    derive_key_from_password, get_base_dir, get_entries_dir, get_entry_path, get_password,
    load_private_key, load_public_key, load_symmetric_key,
};

pub fn cmd_init() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = get_base_dir();
    let entries_dir = base_dir.join("entries");
    fs::create_dir_all(&entries_dir).expect("Failed to create entries directory");

    let password = get_password();
    let argon2_salt = {
        let mut salt = [0u8; 32];
        rand::fill(&mut salt);
        salt
    };

    let symmetric_key = {
        let mut salt = [0u8; 32];
        rand::fill(&mut salt);
        salt
    };
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

    fs::write(base_dir.join("key.key"), cipher_key.serialize())?;
    fs::write(base_dir.join("key.pub"), public_key.as_bytes())?;
    fs::write(base_dir.join("key.sym"), symmetric_key)?;

    println!("Initialized diaria in {}", base_dir.display());
    Ok(())
}

pub fn cmd_add(input: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let salt = load_symmetric_key();
    let public_key = load_public_key();
    let entry_path = get_entry_path();

    let input = if let Some(p) = input {
        fs::read_to_string(p)?
    } else {
        Editor::new().edit("")?.unwrap_or_default()
    };

    let encoded = encode(&public_key, &input, &salt)?;
    fs::write(&entry_path, encoded)?;
    println!("Created entry: {}", entry_path.display());
    Ok(())
}

pub fn cmd_read(filename: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let salt = load_symmetric_key();
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

        let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an entry")
            .items(entries.iter().map(|p| p.display()).collect::<Vec<_>>())
            .interact()?;

        entries[selection].clone()
    };

    let private_key = load_private_key();
    let data = fs::read(&entry_path)?;
    let plaintext = decode(&private_key, &data, &salt)?;
    println!("{}", plaintext);
    Ok(())
}

pub fn cmd_load(directory: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let salt = load_symmetric_key();
    let public_key = load_public_key();
    let entries_dir = get_entries_dir();

    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let content = fs::read_to_string(&path)?;
            let encoded = encode(&public_key, &content, &salt)?;
            let dest_path = entries_dir.join(path.file_name().unwrap());
            fs::write(dest_path, encoded)?;
        }
    }
    println!("Loaded entries from {}", directory.display());
    Ok(())
}

pub fn cmd_dump(directory: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let salt = load_symmetric_key();
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
    Ok(())
}

pub fn cmd_sync() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

pub fn cmd_summarize() -> Result<(), Box<dyn std::error::Error>> {
    let salt = load_symmetric_key();
    let entries_dir = get_entries_dir();
    let private_key = load_private_key();
    let now = Local::now();

    let time_offsets = [1, 7, 30, 365, 365 * 2, 365 * 4, 365 * 8, 365 * 16]
        .map(chrono::Duration::days)
        .map(|x| now - x);

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
    Ok(())
}

pub fn cmd_stats() -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
