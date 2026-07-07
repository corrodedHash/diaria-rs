use std::fs;
use std::path::Path;

use crate::entry::{key_manager::DiariaKeyManager, repository::DiariaEntryRepository, version01::encode};
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

    pub fn execute(&self, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let salt = self.key_manager.load_symmetric_key();
        let public_key = self.key_manager.load_public_key();

        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path)?;
                let encoded = encode(&public_key, &content, &salt)?;
                let name = path.file_name().unwrap().to_string_lossy();
                self.repository.store_entry(&name, &encoded)?;
            }
        }
        self.user_output
            .print(&format!("Loaded entries from {}", directory.display()));
        Ok(())
    }
}
