use std::process::Command as ProcessCommand;

use crate::entry::repository::DiariaMetaRepository;
use crate::stdout_printer::UserOutput;

pub struct Command {
    repo: Box<dyn DiariaMetaRepository>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(repo: Box<dyn DiariaMetaRepository>, user_output: Box<dyn UserOutput>) -> Self {
        Self { repo, user_output }
    }

    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.repo.create_structure();
        let entries_dir = self.repo.get_base_dir().join("entries");

        if !entries_dir.join(".git").exists() {
            self.user_output
                .print(&format!("Not a git repository: {}", entries_dir.display()));
            return Ok(());
        }

        let git_invocations = [
            vec!["add", "*.diaria"],
            vec!["commit", "-m", "Auto-commit entries"],
            vec!["push"],
            vec!["pull"],
        ];
        for args in git_invocations {
            ProcessCommand::new("git")
                .arg("-C")
                .arg(&entries_dir)
                .args(&args)
                .status()?;
        }

        self.user_output.print("Synced entries repository");
        Ok(())
    }
}
