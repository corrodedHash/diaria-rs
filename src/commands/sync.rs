use crate::entry::repository::DiariaMetaRepository;
use crate::util::git_runner::GitRunner;
use crate::util::stdout_printer::UserOutput;

pub struct Command {
    repo: Box<dyn DiariaMetaRepository>,
    git_runner: Box<dyn GitRunner>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(
        repo: Box<dyn DiariaMetaRepository>,
        git_runner: Box<dyn GitRunner>,
        user_output: Box<dyn UserOutput>,
    ) -> Self {
        Self {
            repo,
            git_runner,
            user_output,
        }
    }

    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.repo.create_structure();
        let entries_dir = self.repo.get_base_dir().join("entries");

        if !entries_dir.join(".git").exists() {
            self.user_output
                .print(&format!("Not a git repository: {}", entries_dir.display()));
            return Ok(());
        }

        let entries_arg = entries_dir.to_string_lossy().into_owned();
        let git_invocations: [Vec<&str>; 4] = [
            vec!["-C", &entries_arg, "add", "*.diaria"],
            vec!["-C", &entries_arg, "commit", "-m", "Auto-commit entries"],
            vec!["-C", &entries_arg, "push"],
            vec!["-C", &entries_arg, "pull"],
        ];
        for args in git_invocations {
            self.git_runner.run(&args)?;
        }

        self.user_output.print("Synced entries repository");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::util::git_runner::MockGitRunner;
    use crate::util::stdout_printer::MockUserOutput;
    use std::path::PathBuf;

    mockall::mock! {
        pub Repo {}
        impl DiariaMetaRepository for Repo {
            fn create_structure(&self);
            fn get_base_dir(&self) -> PathBuf;
            fn store_private_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
            fn store_public_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
            fn store_symmetric_key_raw(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
            fn store_manifest_raw(&self, manifest: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
            fn fetch_private_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
            fn fetch_public_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
            fn fetch_symmetric_key_raw(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
            fn fetch_manifest_raw(&self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>>;
        }
    }

    #[test]
    fn noop_when_not_a_git_repo() {
        let mut repo = MockRepo::new();
        repo.expect_create_structure().returning(|| ());
        repo.expect_get_base_dir()
            .returning(|| PathBuf::from("/tmp/non_git_vault"));

        let mut user_output = MockUserOutput::new();
        user_output
            .expect_print()
            .withf(|msg| msg.contains("Not a git repository"))
            .returning(|_| ());

        let git_runner = MockGitRunner::new();

        let cmd = Command::new(Box::new(repo), Box::new(git_runner), Box::new(user_output));
        cmd.execute().unwrap();
    }

    #[test]
    fn runs_git_invocations() {
        let mut repo = MockRepo::new();
        repo.expect_create_structure().returning(|| ());
        repo.expect_get_base_dir()
            .returning(|| PathBuf::from("/tmp/git_vault"));

        // Create the .git directory so the command proceeds past the check
        std::fs::create_dir_all("/tmp/git_vault/entries/.git").ok();

        let mut git_runner = MockGitRunner::new();
        git_runner
            .expect_run()
            .withf(|args| args == ["-C", "/tmp/git_vault/entries", "add", "*.diaria"])
            .returning(|_| Ok(()));
        git_runner
            .expect_run()
            .withf(|args| {
                args == [
                    "-C",
                    "/tmp/git_vault/entries",
                    "commit",
                    "-m",
                    "Auto-commit entries",
                ]
            })
            .returning(|_| Ok(()));
        git_runner
            .expect_run()
            .withf(|args| args == ["-C", "/tmp/git_vault/entries", "push"])
            .returning(|_| Ok(()));
        git_runner
            .expect_run()
            .withf(|args| args == ["-C", "/tmp/git_vault/entries", "pull"])
            .returning(|_| Ok(()));

        let mut user_output = MockUserOutput::new();
        user_output
            .expect_print()
            .withf(|msg| msg.contains("Synced entries repository"))
            .returning(|_| ());

        let cmd = Command::new(Box::new(repo), Box::new(git_runner), Box::new(user_output));
        cmd.execute().unwrap();

        std::fs::remove_dir_all("/tmp/git_vault").ok();
    }
}
