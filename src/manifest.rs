//! Vault manifest and format versioning.
//!
//! The manifest is a small `manifest.toml` at the vault root that records a
//! single integer: the vault format version. The version is the source of
//! truth for the whole setup — the concrete crypto parameters (KDF, AEAD, key
//! algorithm) and the on-disk layout of keys and entries are all *implied* by
//! the version and baked into this binary, never read from the file. That way
//! a hand-edited manifest can only ever select a version this binary knows how
//! to handle; it can never conjure an invalid or weakened crypto config.
//!
//! Migrating between versions is therefore a code concern (a `match` on the
//! version), not a matter of reconciling free-form parameters.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The format version this binary writes and considers current.
pub const CURRENT_VERSION: u32 = 1;

/// Contents of `manifest.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
}

impl Manifest {
    /// A manifest describing the current format version.
    pub fn current() -> Self {
        Self {
            version: CURRENT_VERSION,
        }
    }

    pub fn to_toml(&self) -> String {
        // Serializing a two-field struct to a documented format cannot fail.
        toml::to_string(self).expect("manifest serialization is infallible")
    }

    /// Parse and validate raw manifest bytes, yielding the vault's version.
    ///
    /// Returns [`ManifestError::UnknownVersion`] for a vault written by a newer
    /// binary than this one, so we never operate on a format we don't fully
    /// understand.
    pub fn parse(bytes: &[u8]) -> Result<Self, ManifestError> {
        let text = std::str::from_utf8(bytes).map_err(|_| ManifestError::Malformed)?;
        let manifest: Manifest = toml::from_str(text).map_err(|_| ManifestError::Malformed)?;

        if manifest.version == 0 || manifest.version > CURRENT_VERSION {
            return Err(ManifestError::UnknownVersion(manifest.version));
        }

        Ok(manifest)
    }
}

#[derive(Debug, Error)]
pub enum ManifestError {
    /// No manifest was found. Such a vault predates versioning ("v0"): its keys
    /// and entries were written by a tool that cannot be read here and must be
    /// regenerated.
    #[error(
        "no manifest found: this vault predates versioning and cannot be read; \
         its keys and entries must be regenerated with `diaria init`"
    )]
    LegacyUnversioned,
    /// The manifest declares a version this binary does not know how to handle
    /// (i.e. it was written by a newer release).
    #[error("unsupported vault version {0}: this binary supports up to version {CURRENT_VERSION}")]
    UnknownVersion(u32),
    /// The manifest exists but is not valid UTF-8 TOML with the expected shape.
    #[error("manifest is malformed")]
    Malformed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_current_version() {
        let manifest = Manifest::current();
        let toml = manifest.to_toml();
        let parsed = Manifest::parse(toml.as_bytes()).expect("current manifest parses");
        assert_eq!(parsed, manifest);
        assert_eq!(parsed.version, CURRENT_VERSION);
    }

    #[test]
    fn rejects_future_version() {
        let toml = format!("version = {}\n", CURRENT_VERSION + 1);
        let err = Manifest::parse(toml.as_bytes()).expect_err("future version rejected");
        assert!(matches!(err, ManifestError::UnknownVersion(v) if v == CURRENT_VERSION + 1));
    }

    #[test]
    fn rejects_zero_version() {
        let err = Manifest::parse(b"version = 0\n").expect_err("version 0 rejected");
        assert!(matches!(err, ManifestError::UnknownVersion(0)));
    }

    #[test]
    fn rejects_malformed() {
        assert!(matches!(
            Manifest::parse(b"not toml at all: [[["),
            Err(ManifestError::Malformed)
        ));
        assert!(matches!(
            Manifest::parse(b"other_field = 3\n"),
            Err(ManifestError::Malformed)
        ));
    }
}
