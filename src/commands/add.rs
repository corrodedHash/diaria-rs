use std::fmt::Write;
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

        let symmetric_key = self.key_manager.load_symmetric_key()?;
        let public_key = self.key_manager.load_public_key()?;

        let input: Zeroizing<String> = if let Some(p) = path {
            self.file_loader.load(p)?
        } else {
            Zeroizing::from(self.dialogue_editor.edit("")?)
        };

        if input.trim().is_empty() {
            return Err(Box::new(AddError::EmptyEntry));
        }

        let encoded = encode(&public_key, &input, &symmetric_key)?;

        let entry_id = self.repository.add_entry(&encoded)?;

        // --- text stats ---
        let word_count = input.split_whitespace().count();
        let paragraph_count = input
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .count()
            .max(1);
        let longest_word = input.split_whitespace().max_by_key(|w| w.len());

        // --- size comparison ---
        let new_size = u64::try_from(encoded.len()).unwrap_or(u64::MAX);
        let existing_sizes: Vec<u64> = self
            .repository
            .list_entry_metadata()
            .iter()
            .map(|m| m.size)
            .collect();
        let n_larger = existing_sizes.iter().filter(|&&s| s > new_size).count();
        let total = existing_sizes.len();

        let mut msg = format!("Created entry: {entry_id}");
        let _ = write!(
            msg,
            "\nStats: {word_count} words, {paragraph_count} paragraphs"
        );
        if let Some(w) = longest_word {
            let _ = write!(msg, ", longest word: \"{w}\"");
        }

        #[allow(clippy::as_conversions)]
        let size_kb = new_size as f64 / 1024.0;
        let _ = write!(msg, "\nEncrypted size: {size_kb:.1} KB");
        if total == 0 {
            let _ = write!(msg, " — only entry");
        } else {
            #[allow(clippy::as_conversions)]
            let pct = (total.saturating_sub(n_larger)) as f64 / total as f64 * 100.0;
            let _ = write!(
                msg,
                " — larger than {pct:.0}% of entries ({n_larger} larger)"
            );
        }

        self.user_output.print(&msg);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::entry::key_manager::MockDiariaKeyManager;
    use crate::entry::repository::MockDiariaEntryRepository;
    use crate::entry::version01::generate_keypair;
    use crate::manifest::ManifestError;
    use crate::util::dialogue_editor::MockDialogueEditor;
    use crate::util::file_loader::MockFileLoader;
    use crate::util::stdout_printer::MockUserOutput;

    /// Helper: build a [`Command`] wired to mocks, then call `execute`.
    ///
    /// Each test sets up only the expectations it cares about; any unexpected
    /// mock call (e.g. `add_entry` when `encode` should have failed first) will
    /// cause a mockall panic and fail the test.
    fn execute_with_mocks(
        repository: MockDiariaEntryRepository,
        key_manager: MockDiariaKeyManager,
        file_loader: MockFileLoader,
        dialogue_editor: MockDialogueEditor,
        user_output: MockUserOutput,
        input: Option<&Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let command = Command::new(
            Box::new(repository),
            Box::new(key_manager),
            Box::new(file_loader),
            Box::new(dialogue_editor),
            Box::new(user_output),
        );
        command.execute(input)
    }

    #[test]
    fn rejects_whitespace_only_entry() {
        let mut key_manager = MockDiariaKeyManager::new();
        key_manager
            .expect_load_manifest_version()
            .returning(|| Ok(1));
        // Keys are loaded *before* input, so they must be set up even for the
        // empty-entry path (which rejects after input is gathered).
        key_manager
            .expect_load_symmetric_key()
            .returning(|| Ok([0u8; 32]));
        let (_, pk) = generate_keypair();
        key_manager
            .expect_load_public_key()
            .returning(move || Ok(pk));

        let mut file_loader = MockFileLoader::new();
        file_loader
            .expect_load()
            .returning(|_| Ok(Zeroizing::from(" \t\r\n".to_string())));

        let repository = MockDiariaEntryRepository::new();
        let dialogue_editor = MockDialogueEditor::new();
        let user_output = MockUserOutput::new();

        let err = execute_with_mocks(
            repository,
            key_manager,
            file_loader,
            dialogue_editor,
            user_output,
            Some(Path::new("ignored")),
        )
        .expect_err("whitespace-only entry should be rejected");
        assert!(
            err.downcast_ref::<AddError>()
                .is_some_and(|e| matches!(e, AddError::EmptyEntry)),
            "expected EmptyEntry, got: {err}"
        );
    }

    #[test]
    fn manifest_version_error_propagates_before_input() {
        let mut key_manager: MockDiariaKeyManager = MockDiariaKeyManager::new();
        key_manager
            .expect_load_manifest_version()
            .returning(|| Err(ManifestError::LegacyUnversioned));

        // No other interaction should happen — no file loading, no editor, no
        // storage. All other mocks are left without expectations.
        let file_loader = MockFileLoader::new();
        let dialogue_editor = MockDialogueEditor::new();
        let repository = MockDiariaEntryRepository::new();
        let user_output = MockUserOutput::new();

        let err = execute_with_mocks(
            repository,
            key_manager,
            file_loader,
            dialogue_editor,
            user_output,
            Some(Path::new("ignored")),
        )
        .expect_err("manifest error should propagate");
        assert!(
            err.downcast_ref::<ManifestError>()
                .is_some_and(|e| matches!(e, ManifestError::LegacyUnversioned)),
            "expected LegacyUnversioned, got: {err}"
        );
    }

    #[test]
    fn add_entry_error_after_input_loses_entry() {
        let mut key_manager = MockDiariaKeyManager::new();
        key_manager
            .expect_load_manifest_version()
            .returning(|| Ok(1));
        key_manager
            .expect_load_symmetric_key()
            .returning(|| Ok([0u8; 32]));
        let (_, pk) = generate_keypair();
        key_manager
            .expect_load_public_key()
            .returning(move || Ok(pk));

        let mut file_loader = MockFileLoader::new();
        file_loader
            .expect_load()
            .returning(|_| Ok(Zeroizing::from("hello, diary".to_string())));

        let mut repository = MockDiariaEntryRepository::new();
        repository.expect_add_entry().returning(|_| {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            )))
        });

        // The entry text was fully processed (compressed, encrypted, enveloped)
        // but the file write failed — the user's text is lost. We must never
        // print "Created entry: …" on a failed write.
        let user_output = MockUserOutput::new();
        let dialogue_editor = MockDialogueEditor::new();

        let err = execute_with_mocks(
            repository,
            key_manager,
            file_loader,
            dialogue_editor,
            user_output,
            Some(Path::new("ignored")),
        )
        .expect_err("add_entry error should propagate");
        assert!(
            err.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied),
            "expected io::PermissionDenied, got: {err}"
        );
    }
}
