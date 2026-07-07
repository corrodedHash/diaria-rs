use chrono::Local;

use crate::entry::{key_manager::DiariaKeyManager, repository::DiariaEntryRepository, version01::decode};
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

    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let salt = self.key_manager.load_symmetric_key();
        let private_key = self.key_manager.load_private_key();
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
                    self.user_output
                        .print(&format!("=== Entry from {} ===\n{}\n", date_str, plaintext));
                }
            }
        }
        Ok(())
    }
}
