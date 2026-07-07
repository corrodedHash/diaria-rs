use std::path::Path;

use dialoguer::Editor;

use crate::{
    entry::{encode, key_manager::DiariaKeyManager, repository::DiariaEntryRepository},
    file_loader::FileLoader,
    stdout_printer::UserOutput,
};

pub struct Command {
    repository: Box<dyn DiariaEntryRepository>,
    key_manager: Box<dyn DiariaKeyManager>,
    file_loader: Box<dyn FileLoader>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(
        repository: Box<dyn DiariaEntryRepository>,
        key_manager: Box<dyn DiariaKeyManager>,
        file_loader: Box<dyn FileLoader>,
        user_output: Box<dyn UserOutput>,
    ) -> Self {
        Self {
            repository,
            key_manager,
            file_loader,
            user_output,
        }
    }

    pub fn execute(&self, input: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        self.key_manager.load_manifest_version()?;

        let salt = self.key_manager.load_symmetric_key();
        let public_key = self.key_manager.load_public_key();

        let input = if let Some(p) = input {
            self.file_loader.load(p)?
        } else {
            Editor::new().edit("")?.unwrap_or_default()
        };

        let encoded = encode(&public_key, &input, &salt)?;

        let entry_id = self.repository.add_entry(&encoded)?;
        self.user_output
            .print(&format!("Created entry: {}", entry_id));
        Ok(())
    }
}
