use std::path::PathBuf;
use std::{fs, path::Path};

use chrono::Local;
use xdg::BaseDirectories;

pub struct EntryMetadata {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub size: u64,
}

#[mockall::automock]
pub trait DiariaEntryRepository {
    fn list_entries(&self) -> Vec<PathBuf>;
    fn list_entry_metadata(&self) -> Vec<EntryMetadata>;
    /// Stores an entry under a generated timestamp id, returning that id.
    fn add_entry(&self, entry: &[u8]) -> Result<String, Box<dyn std::error::Error>>;
    /// Stores an entry under an explicit file name (used when importing existing entries).
    fn store_entry(&self, name: &str, entry: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn read_entry(&self, entry_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

#[mockall::automock]
pub trait DiariaMetaRepository {
    fn create_structure(&self);
    fn get_base_dir(&self) -> PathBuf;

    fn store_private_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn store_public_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn store_symmetric_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn store_manifest_raw(&self, manifest: &[u8]) -> Result<(), Box<dyn std::error::Error>>;

    fn fetch_private_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn fetch_public_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn fetch_symmetric_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    /// Read the raw `manifest.toml` bytes, or `None` if the vault has no
    /// manifest (a legacy, unversioned "v0" vault).
    fn fetch_manifest_raw(&self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>>;
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
        self.store_entry(&format!("{}.diaria", timestamp), entry)?;
        Ok(timestamp)
    }

    fn store_entry(&self, name: &str, entry: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let entry_path = self.get_base_dir().join("entries").join(name);
        std::fs::write(&entry_path, entry)?;
        Ok(())
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
                            timestamp: timestamp.with_timezone(&Local),
                            size: metadata.len(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
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

    fn store_manifest_raw(&self, manifest: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(self.get_base_dir().join("manifest.toml"), manifest)?;
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

    fn fetch_manifest_raw(&self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        match std::fs::read(self.get_base_dir().join("manifest.toml")) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}
