use std::path::Path;
use zeroize::Zeroizing;

pub struct RealFileLoader;

#[mockall::automock]
pub trait FileLoader {
    fn load(&self, path: &Path) -> std::io::Result<Zeroizing<String>>;
}

impl FileLoader for RealFileLoader {
    fn load(&self, path: &Path) -> std::io::Result<Zeroizing<String>> {
        std::fs::read_to_string(path).map(Zeroizing::from)
    }
}
