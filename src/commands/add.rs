use std::path::Path;

use thiserror::Error;
use zeroize::Zeroizing;

use crate::{
    entry::{encode, key_manager::DiariaKeyManager, repository::DiariaEntryRepository},
    util::dialogue_editor::DialogueEditor,
    util::file_loader::FileLoader,
    util::stdout_printer::UserOutput,
};

#[derive(Debug, Error)]
pub enum AddError {
    /// The entry text is empty or only whitespace; there is nothing to store.
    #[error("refusing to add an empty entry")]
    EmptyEntry,
}

pub struct Command {
    repository: Box<dyn DiariaEntryRepository>,
    key_manager: Box<dyn DiariaKeyManager>,
    file_loader: Box<dyn FileLoader>,
    dialogue_editor: Box<dyn DialogueEditor>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(
        repository: Box<dyn DiariaEntryRepository>,
        key_manager: Box<dyn DiariaKeyManager>,
        file_loader: Box<dyn FileLoader>,
        dialogue_editor: Box<dyn DialogueEditor>,
        user_output: Box<dyn UserOutput>,
    ) -> Self {
        Self {
            repository,
            key_manager,
            file_loader,
            dialogue_editor,
            user_output,
        }
    }

    pub fn execute(&self, path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        self.key_manager.load_manifest_version()?;

        let input: Zeroizing<String> = if let Some(p) = path {
            self.file_loader.load(p)?
        } else {
            Zeroizing::from(self.dialogue_editor.edit("")?)
        };

        // Reject empty or whitespace-only entries before doing any crypto, so we
        // never store a "blank" entry.
        if input.trim().is_empty() {
            return Err(Box::new(AddError::EmptyEntry));
        }

        let symmetric_key = self.key_manager.load_symmetric_key()?;
        let public_key = self.key_manager.load_public_key()?;

        let encoded = encode(&public_key, &input, &symmetric_key)?;

        let entry_id = self.repository.add_entry(&encoded)?;
        self.user_output
            .print(&format!("Created entry: {entry_id}"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::key_manager::MockDiariaKeyManager;
    use crate::entry::repository::MockDiariaEntryRepository;
    use crate::util::dialogue_editor::MockDialogueEditor;
    use crate::util::file_loader::MockFileLoader;
    use crate::util::stdout_printer::MockUserOutput;

    #[test]
    fn rejects_whitespace_only_entry() {
        let mut key_manager = MockDiariaKeyManager::new();
        key_manager
            .expect_load_manifest_version()
            .returning(|| Ok(1));

        let mut file_loader = MockFileLoader::new();
        file_loader
            .expect_load()
            .returning(|_| Ok(Zeroizing::from(" \t\r\n".to_string())));

        // No entry may be stored and nothing may be printed for an empty entry;
        // leaving these mocks without expectations makes any such call fail.
        let repository = MockDiariaEntryRepository::new();
        let dialogue_editor = MockDialogueEditor::new();
        let user_output = MockUserOutput::new();

        let command = Command::new(
            Box::new(repository),
            Box::new(key_manager),
            Box::new(file_loader),
            Box::new(dialogue_editor),
            Box::new(user_output),
        );

        let err = command
            .execute(Some(Path::new("ignored")))
            .expect_err("whitespace-only entry should be rejected");
        assert!(
            err.downcast_ref::<AddError>()
                .is_some_and(|e| matches!(e, AddError::EmptyEntry)),
            "expected EmptyEntry, got: {err}"
        );
    }
}
