use std::path::Path;

pub struct RealFileLoader;

#[mockall::automock]
pub trait FileLoader {
    fn load(&self, path: &Path) -> std::io::Result<String>;
}

impl FileLoader for RealFileLoader {
    fn load(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}
