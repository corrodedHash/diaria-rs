use dialoguer::Password;

use crate::environment::Environment;

/// Environment variable that, when set, supplies the encryption password
/// non-interactively (bypassing the terminal prompt). Primarily for the
/// end-to-end tests, which drive the CLI without a TTY.
pub const PASSWORD_ENV: &str = "DIARIA_PASSWORD";

pub struct TerminalPasswordService {
    environment: Box<dyn Environment>,
}

impl TerminalPasswordService {
    pub fn new(environment: Box<dyn Environment>) -> Self {
        Self { environment }
    }
}

#[mockall::automock]
pub trait PasswordService {
    fn get_password(&self) -> String;
}

impl PasswordService for TerminalPasswordService {
    fn get_password(&self) -> String {
        if let Some(password) = self.environment.get(PASSWORD_ENV) {
            return password;
        }
        Password::new()
            .with_prompt("Enter encryption password")
            .interact()
            .expect("Failed to read password")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::MockEnvironment;

    #[test]
    fn uses_password_from_environment_when_set() {
        let mut env = MockEnvironment::new();
        env.expect_get()
            .withf(|key| key == PASSWORD_ENV)
            .return_const(Some("s3cret".to_string()));

        let service = TerminalPasswordService::new(Box::new(env));
        assert_eq!(service.get_password(), "s3cret");
    }
}
