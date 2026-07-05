use dialoguer::Password;

pub struct TerminalPasswordService {}

#[mockall::automock]
pub trait PasswordService {
    fn get_password(&self) -> String;
}

impl PasswordService for TerminalPasswordService {
    fn get_password(&self) -> String {
        Password::new()
            .with_prompt("Enter encryption password")
            .interact()
            .expect("Failed to read password")
    }
}
