//! Process environment access, behind a trait so it can be injected.
//!
//! Reading `std::env` directly from business logic makes that logic awkward to
//! unit-test (env vars are process-global and racy under parallel tests). Every
//! environment lookup goes through [`Environment`] instead, so tests can supply
//! a deterministic fake via `MockEnvironment`.

/// Read-only access to environment variables.
#[mockall::automock]
pub trait Environment {
    /// Returns the value of the environment variable `key`, or `None` if it is
    /// unset or not valid Unicode.
    fn get(&self, key: &str) -> Option<String>;
}

/// The real environment, backed by [`std::env::var`].
pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
