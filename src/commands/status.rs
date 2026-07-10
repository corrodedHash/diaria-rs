use crate::entry::repository::{DiariaEntryRepository, DiariaMetaRepository};
use crate::manifest::{Manifest, ManifestError};
use crate::util::stdout_printer::UserOutput;

pub struct Command {
    meta_repo: Box<dyn DiariaMetaRepository>,
    entry_repo: Box<dyn DiariaEntryRepository>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(
        meta_repo: Box<dyn DiariaMetaRepository>,
        entry_repo: Box<dyn DiariaEntryRepository>,
        user_output: Box<dyn UserOutput>,
    ) -> Self {
        Self {
            meta_repo,
            entry_repo,
            user_output,
        }
    }

    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let base_dir = self.meta_repo.get_base_dir();
        let entries_dir = base_dir.join("entries");

        // The manifest is the marker that `init` ran: a present, valid manifest
        // means the vault is set up. Its parsed version is the vault format
        // version. Both are derived from a single read so the labels can't
        // drift apart.
        let (vault_version, vault_setup) = match self.meta_repo.fetch_manifest_raw() {
            Ok(None) => (
                "unknown (no manifest)".to_string(),
                "not initialized (no manifest)".to_string(),
            ),
            Ok(Some(bytes)) => match Manifest::parse(&bytes) {
                Ok(manifest) => (manifest.version.to_string(), "initialized".to_string()),
                Err(ManifestError::UnknownVersion(v)) => (
                    format!("unknown (unsupported version {v})"),
                    format!("not initialized (unsupported manifest version {v})"),
                ),
                Err(ManifestError::Malformed) => (
                    "unknown (malformed manifest)".to_string(),
                    "not initialized (malformed manifest)".to_string(),
                ),
                Err(ManifestError::LegacyUnversioned) => (
                    "unknown (no manifest)".to_string(),
                    "not initialized (no manifest)".to_string(),
                ),
            },
            Err(_) => (
                "unknown (cannot read manifest)".to_string(),
                "not initialized (cannot read manifest)".to_string(),
            ),
        };

        // Key presence is reported per-file; a failed read is treated as
        // missing so a half-initialized vault shows which material is absent.
        let private_found = self.meta_repo.fetch_private_key_raw().is_ok();
        let public_found = self.meta_repo.fetch_public_key_raw().is_ok();
        let symmetric_found = self.meta_repo.fetch_symmetric_key_raw().is_ok();

        // `list_entry_metadata` filters to real diary entries (it parses each
        // filename's timestamp), so incidental files like a `.git` directory
        // under `entries/` don't inflate the count.
        let entry_count = self.entry_repo.list_entry_metadata().len();

        // `sync` operates on a git repo inside `entries/`; the presence of
        // `.git` there is what makes sync usable.
        let git_sync = if entries_dir.join(".git").exists() {
            "configured"
        } else {
            "not configured"
        };

        let binary_version = env!("CARGO_PKG_VERSION");
        let found_or_missing = |found: bool| if found { "found" } else { "missing" };

        let mut report = String::new();
        report.push_str(&format!("diaria {binary_version}\n\n"));
        report.push_str(&format!("Vault: {}\n", base_dir.display()));
        report.push_str(&format!("Entries: {}\n\n", entries_dir.display()));
        report.push_str(&format!("Vault format version: {vault_version}\n"));
        report.push_str(&format!("Setup: {vault_setup}\n\n"));
        report.push_str("Keys:\n");
        report.push_str(&format!(
            "  private key:   {}\n",
            found_or_missing(private_found)
        ));
        report.push_str(&format!(
            "  public key:    {}\n",
            found_or_missing(public_found)
        ));
        report.push_str(&format!(
            "  symmetric key: {}\n\n",
            found_or_missing(symmetric_found)
        ));
        report.push_str(&format!("Entries: {entry_count}\n\n"));
        report.push_str(&format!("Git sync: {git_sync}"));

        self.user_output.print(&report);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::TimeZone as _;

    use crate::entry::repository::{
        EntryMetadata, MockDiariaEntryRepository, MockDiariaMetaRepository,
    };
    use crate::manifest::Manifest;
    use crate::util::stdout_printer::MockUserOutput;

    use super::*;

    fn io_not_found() -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
    }

    fn entry_at(timestamp: &str) -> EntryMetadata {
        EntryMetadata {
            timestamp: chrono::Local
                .from_local_datetime(
                    &chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S").unwrap(),
                )
                .unwrap(),
            size: 1024,
        }
    }

    #[test]
    fn status_reports_an_initialized_vault() {
        let mut meta = MockDiariaMetaRepository::new();
        meta.expect_get_base_dir()
            .returning(|| PathBuf::from("/tmp/diaria"));
        meta.expect_fetch_manifest_raw()
            .returning(|| Ok(Some(Manifest::current().to_toml().into_bytes())));
        meta.expect_fetch_private_key_raw()
            .returning(|| Ok(vec![1u8; 56]));
        meta.expect_fetch_public_key_raw()
            .returning(|| Ok(vec![2u8; 56]));
        meta.expect_fetch_symmetric_key_raw()
            .returning(|| Ok(vec![3u8; 32]));

        let mut entries = MockDiariaEntryRepository::new();
        entries.expect_list_entry_metadata().returning(|| {
            vec![
                entry_at("2026-01-01T00:00:00"),
                entry_at("2026-01-02T00:00:00"),
            ]
        });

        let mut out = MockUserOutput::new();
        out.expect_print()
            .withf(|text| {
                text.contains("Vault format version: 1")
                    && text.contains("Setup: initialized")
                    && text.contains("found")
                    && !text.contains("missing")
                    && text.contains("Entries: 2")
            })
            .return_const(());

        Command::new(Box::new(meta), Box::new(entries), Box::new(out))
            .execute()
            .expect("status should succeed");
    }

    #[test]
    fn status_reports_an_uninitialized_vault() {
        let mut meta = MockDiariaMetaRepository::new();
        meta.expect_get_base_dir()
            .returning(|| PathBuf::from("/tmp/diaria"));
        meta.expect_fetch_manifest_raw().returning(|| Ok(None));
        meta.expect_fetch_private_key_raw()
            .returning(|| Err(io_not_found()));
        meta.expect_fetch_public_key_raw()
            .returning(|| Err(io_not_found()));
        meta.expect_fetch_symmetric_key_raw()
            .returning(|| Err(io_not_found()));

        let mut entries = MockDiariaEntryRepository::new();
        entries
            .expect_list_entry_metadata()
            .returning(std::vec::Vec::new);

        let mut out = MockUserOutput::new();
        out.expect_print()
            .withf(|text| {
                text.contains("Setup: not initialized")
                    && text.contains("missing")
                    && !text.contains("found")
                    && text.contains("Entries: 0")
            })
            .return_const(());

        Command::new(Box::new(meta), Box::new(entries), Box::new(out))
            .execute()
            .expect("status should succeed");
    }
}
