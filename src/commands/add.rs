use std::path::Path;

use dialoguer::Editor;

use crate::{
    entry::{
        encode,
        key_manager::{DiariaKeyManager, FsKeyManagerDefault},
        repository::{DiariaEntryRepository, DiariaFsRepository},
    },
    file_loader::{FileLoader, RealFileLoader},
};

pub struct Command<T: DiariaEntryRepository, KM: DiariaKeyManager, F: FileLoader> {
    repository: T,
    key_manager: KM,
    file_loader: F,
}

impl Default for Command<DiariaFsRepository, FsKeyManagerDefault, RealFileLoader> {
    fn default() -> Self {
        Self {
            repository: DiariaFsRepository {},
            key_manager: FsKeyManagerDefault::default(),
            file_loader: RealFileLoader,
        }
    }
}

impl<T: DiariaEntryRepository, KM: DiariaKeyManager, F: FileLoader> Command<T, KM, F> {
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
        println!("Created entry: {}", entry_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
