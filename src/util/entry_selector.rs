use std::path::PathBuf;

use dialoguer::FuzzySelect;

#[mockall::automock]
pub trait EntrySelector {
    fn select(&self, entries: &[PathBuf]) -> Result<usize, Box<dyn std::error::Error>>;
}

pub struct RealEntrySelector;

impl EntrySelector for RealEntrySelector {
    fn select(&self, entries: &[PathBuf]) -> Result<usize, Box<dyn std::error::Error>> {
        let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an entry")
            .items(entries.iter().map(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_else(|| p.to_str().unwrap_or("<invalid>"))
            }))
            .interact()?;
        Ok(selection)
    }
}
