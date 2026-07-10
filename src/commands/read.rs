use std::path::Path;

use dialoguer::FuzzySelect;

use crate::entry::{decode, key_manager::DiariaKeyManager, repository::DiariaEntryRepository};
use crate::util::stdout_printer::UserOutput;

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

    pub fn execute(&self, filename: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
        self.key_manager.load_manifest_version()?;

        let entry_path = if let Some(f) = filename {
            f.to_path_buf()
        } else {
            let Some(entry_path) = self.user_entry_selection()? else {
                return Ok(());
            };
            entry_path
        };

        let salt = self.key_manager.load_symmetric_key();
        let private_key = self.key_manager.load_private_key()?;
        let data = self.repository.read_entry(&entry_path)?;
        let plaintext = decode(&private_key, &data, &salt)?;
        self.user_output.print(&plaintext);
        Ok(())
    }

    fn user_entry_selection(
        &self,
    ) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error + 'static>> {
        let entries = self.repository.list_entries();
        if entries.is_empty() {
            self.user_output.print("No entries found");
            return Ok(None);
        }
        let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an entry")
            .items(entries.iter().map(|p| p.display()).collect::<Vec<_>>())
            .interact()?;
        Ok(Some(entries[selection].clone()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entry::{
            key_manager::FsKeyManager,
            repository::{MockDiariaEntryRepository, MockDiariaMetaRepository},
        },
        util::password::MockPasswordService,
        util::stdout_printer::MockUserOutput,
    };

    use super::*;

    const CIPHERTEXT: &[u8] = include_bytes!("testdata/entries/2026-06-21T16:50:46.diaria");
    const SYMKEY: &[u8] = include_bytes!("testdata/key.sym");
    const PRIVATE_KEY: &[u8] = include_bytes!("testdata/key.key");
    const PUBLIC_KEY: &[u8] = include_bytes!("testdata/key.pub");
    const MANIFEST: &[u8] = include_bytes!("testdata/manifest.toml");
    const PLAINTEXT: &str = "Hello";

    #[test]
    fn test_longterm() {
        let mut repo = MockDiariaEntryRepository::new();
        repo.expect_read_entry()
            .returning(|_| Ok(CIPHERTEXT.to_vec()));

        let mut diaria_meta_repo = MockDiariaMetaRepository::new();
        diaria_meta_repo
            .expect_fetch_private_key_raw()
            .returning(|| Ok(PRIVATE_KEY.to_vec()));
        diaria_meta_repo
            .expect_fetch_public_key_raw()
            .returning(|| Ok(PUBLIC_KEY.to_vec()));
        diaria_meta_repo
            .expect_fetch_symmetric_key_raw()
            .returning(|| Ok(SYMKEY.to_vec()));
        diaria_meta_repo
            .expect_fetch_manifest_raw()
            .returning(|| Ok(Some(MANIFEST.to_vec())));

        let mut password_service = MockPasswordService::new();
        password_service
            .expect_get_password()
            .return_const(zeroize::Zeroizing::from("test".to_string()));

        let mut user_output_service = MockUserOutput::new();
        user_output_service
            .expect_print()
            .withf(|text| text == PLAINTEXT)
            .return_const(());

        let key_manager = FsKeyManager::new(Box::new(diaria_meta_repo), Box::new(password_service));
        Command::new(
            Box::new(repo),
            Box::new(key_manager),
            Box::new(user_output_service),
        )
        .execute(Some(Path::new("testdata/entry1.diaria")))
        .expect("Failed to execute command");
    }

    /// A vault with no manifest is a legacy "v0" vault and must be refused
    /// before any decryption is attempted.
    #[test]
    fn test_legacy_vault_without_manifest_is_rejected() {
        let repo = MockDiariaEntryRepository::new();

        let mut diaria_meta_repo = MockDiariaMetaRepository::new();
        diaria_meta_repo
            .expect_fetch_manifest_raw()
            .returning(|| Ok(None));

        let password_service = MockPasswordService::new();
        let user_output_service = MockUserOutput::new();

        let key_manager = FsKeyManager::new(Box::new(diaria_meta_repo), Box::new(password_service));
        let result = Command::new(
            Box::new(repo),
            Box::new(key_manager),
            Box::new(user_output_service),
        )
        .execute(Some(Path::new("testdata/entry1.diaria")));

        let err = result.expect_err("legacy vault should be rejected");
        assert!(
            err.downcast_ref::<crate::manifest::ManifestError>()
                .is_some_and(|e| matches!(e, crate::manifest::ManifestError::LegacyUnversioned)),
            "expected LegacyUnversioned, got: {err}"
        );
    }
}
