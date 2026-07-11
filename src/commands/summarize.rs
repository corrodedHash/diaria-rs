use chrono::Local;
use zeroize::Zeroizing;

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

    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.key_manager.load_manifest_version()?;

        let salt = self.key_manager.load_symmetric_key()?;
        let private_key = self.key_manager.load_private_key()?;
        let now = Local::now();

        let time_offsets = [1, 7, 30, 365, 365 * 2, 365 * 4, 365 * 8, 365 * 16]
            .map(chrono::Duration::days)
            .map(|x| now - x);

        let entries = self.repository.list_entries();
        for offset in time_offsets {
            let date_str = offset.format("%Y-%m-%d").to_string();

            for path in &entries {
                if let Some(name) = path.file_name()
                    && let Some(name_str) = name.to_str()
                    && name_str.contains(&date_str)
                {
                    let data = self.repository.read_entry(path)?;
                    let plaintext = decode(&private_key, &data, &salt)?;
                    let message = Zeroizing::from(format!(
                        "=== Entry from {} ===\n{}\n",
                        date_str,
                        plaintext.as_str()
                    ));
                    self.user_output.print(message.as_str());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::encode;
    use crate::entry::key_manager::MockDiariaKeyManager;
    use crate::entry::repository::MockDiariaEntryRepository;
    use crate::entry::version01::{SymmetricKey, generate_keypair};
    use crate::util::stdout_printer::MockUserOutput;
    use std::path::PathBuf;

    /// `summarize` selects entries whose filename carries a date exactly
    /// now-{1,7,30,…} days old, then decrypts and prints them. Given one entry
    /// dated seven days ago (a real offset) and one dated far outside any
    /// window, only the former may be read and printed.
    #[test]
    fn decodes_only_entries_dated_at_a_summary_offset() {
        let (private_key, public_key) = generate_keypair();
        let salt: SymmetricKey = [7u8; 32];

        const MATCHING_PLAINTEXT: &str = "the entry from seven days ago";
        let encoded = encode(&public_key, MATCHING_PLAINTEXT, &salt).expect("encode");

        // A filename containing the %Y-%m-%d of seven days ago — one of the
        // fixed summary offsets. The other date (year 2000) is decades before
        // the oldest offset (~16 years back), so it can never match.
        let seven_days_ago = (Local::now() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        let matching = PathBuf::from(format!("{seven_days_ago}T12:00:00.diaria"));
        let non_matching = PathBuf::from("2000-01-01T00:00:00.diaria");

        let entries = vec![matching.clone(), non_matching];
        let mut repo = MockDiariaEntryRepository::new();
        repo.expect_list_entries().return_once(move || entries);
        // Only the matching entry may be read; anything else fails the mock.
        repo.expect_read_entry()
            .withf(move |p| p == matching.as_path())
            .times(1)
            .returning(move |_| Ok(encoded.clone()));

        let mut key_manager = MockDiariaKeyManager::new();
        key_manager
            .expect_load_manifest_version()
            .returning(|| Ok(1));
        key_manager
            .expect_load_symmetric_key()
            .returning(move || Ok(salt));
        let private_key_bytes = *private_key.as_bytes();
        key_manager
            .expect_load_private_key()
            .returning(move || Ok(Zeroizing::new(private_key_bytes)));

        // Exactly one print, and it must carry the matching entry's plaintext.
        let mut user_output = MockUserOutput::new();
        user_output
            .expect_print()
            .withf(|text| text.contains(MATCHING_PLAINTEXT))
            .times(1)
            .return_const(());

        Command::new(Box::new(repo), Box::new(key_manager), Box::new(user_output))
            .execute()
            .expect("summarize should succeed");
    }
}
