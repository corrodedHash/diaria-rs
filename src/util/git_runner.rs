use std::process::Command;

#[allow(clippy::needless_lifetimes)]
#[mockall::automock]
pub trait GitRunner {
    fn run<'a>(&self, args: &[&'a str]) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct RealGitRunner;

#[allow(clippy::needless_lifetimes)]
impl GitRunner for RealGitRunner {
    fn run<'a>(&self, args: &[&'a str]) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("git").args(args).status()?;
        Ok(())
    }
}
