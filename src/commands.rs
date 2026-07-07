use crate::entry::{decode, encode};
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::{get_entries_dir, load_private_key, load_public_key, load_symmetric_key};

mod add;
mod init;
mod read;
mod stats;

pub use add::Command as CmdAdd;
pub use init::Command as CmdInit;
pub use read::Command as CmdRead;
pub use stats::Command as CmdStats;

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
