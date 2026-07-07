use std::fs;
use std::path::PathBuf;

use crate::entry::{decode, key_manager::DiariaKeyManager, repository::DiariaEntryRepository};
use crate::stdout_printer::UserOutput;

pub struct Command {
    repository: Box<dyn DiariaEntryRepository>,
    key_manager: Box<dyn DiariaKeyManager>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(
        repository: Box<dyn DiariaEntryRepository>,
        key_manager: Box<dyn DiariaKeyManager>,
        user_output: Box<dyn UserOutput>,
    ) -> Self {
        Self {
            repository,
            key_manager,
            user_output,
        }
    }

    pub fn execute(&self, directory: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        self.key_manager.load_manifest_version()?;

        let salt = self.key_manager.load_symmetric_key();
        let output_dir = directory.unwrap_or_else(|| PathBuf::from("./dump"));
        fs::create_dir_all(&output_dir)?;

        let private_key = self.key_manager.load_private_key()?;
        for path in self.repository.list_entries() {
            if path.extension().is_some_and(|ext| ext == "diaria") {
                let data = self.repository.read_entry(&path)?;
                let plaintext = decode(&private_key, &data, &salt)?;
                let dest_path = output_dir.join(path.file_stem().unwrap());
                fs::write(dest_path, plaintext)?;
            }
        }
        self.user_output
            .print(&format!("Dumped entries to {}", output_dir.display()));
        Ok(())
    }
}
