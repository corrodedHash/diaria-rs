use std::path::PathBuf;
use std::{fs, path::Path};

use chrono::Local;
use xdg::BaseDirectories;

pub struct EntryMetadata {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub size: u64,
}

#[mockall::automock]
pub trait DiariaEntryRepository {
    fn list_entries(&self) -> Vec<PathBuf>;
    fn list_entry_metadata(&self) -> Vec<EntryMetadata>;
    fn add_entry(&self, entry: &[u8]) -> Result<String, Box<dyn std::error::Error>>;
    fn delete_entry(&self, entry_id: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn read_entry(&self, entry_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

#[mockall::automock]
pub trait DiariaMetaRepository {
    fn create_structure(&self);
    fn get_base_dir(&self) -> PathBuf;

    fn store_private_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn store_public_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn store_symmetric_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;

    fn fetch_private_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn fetch_public_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn fetch_symmetric_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}
pub struct DiariaFsRepository {}

impl DiariaEntryRepository for DiariaFsRepository {
    fn list_entries(&self) -> Vec<PathBuf> {
        let entries_dir = self.get_base_dir().join("entries");
        fs::read_dir(&entries_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn add_entry(&self, entry: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let base_dir = self.get_base_dir();
        let entries_dir = base_dir.join("entries");
        let entry_path = entries_dir.join(format!("{}.diaria", timestamp));
        std::fs::write(&entry_path, entry)?;
        Ok(timestamp)
    }

    fn list_entry_metadata(&self) -> Vec<EntryMetadata> {
        let entries_dir = self.get_base_dir().join("entries");
        fs::read_dir(&entries_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let metadata = e.metadata().ok()?;
                        let binding = e.file_name();
                        let timestamp = binding.to_str()?.split('.').next()?;
                        let timestamp =
                            chrono::DateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S")
                                .ok()?;
                        Some(EntryMetadata {
                            id: e.file_name().to_string_lossy().to_string(),
                            timestamp: timestamp.with_timezone(&Local),
                            size: metadata.len(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn delete_entry(&self, _entry_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    fn read_entry(&self, entry_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(fs::read(entry_path)?)
    }
}

impl DiariaMetaRepository for DiariaFsRepository {
    fn get_base_dir(&self) -> PathBuf {
        BaseDirectories::with_prefix("diaria")
            .get_data_home()
            .expect("Failed to get base dir")
    }

    fn create_structure(&self) {
        let base_dir = self.get_base_dir();
        let entries_dir = base_dir.join("entries");
        std::fs::create_dir_all(entries_dir).expect("Failed to create entries directory");
    }

    fn store_private_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(self.get_base_dir().join("key.key"), key)?;
        Ok(())
    }

    fn store_public_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(self.get_base_dir().join("key.pub"), key)?;
        Ok(())
    }

    fn store_symmetric_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(self.get_base_dir().join("key.sym"), key)?;
        Ok(())
    }

    fn fetch_private_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(self.get_base_dir().join("key.key"))?)
    }

    fn fetch_public_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(self.get_base_dir().join("key.pub"))?)
    }

    fn fetch_symmetric_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(self.get_base_dir().join("key.sym"))?)
    }
}
