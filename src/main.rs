use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

mod commands;
mod crypto;
mod di;
mod entry;
mod manifest;
mod util;

use di::Di;

#[derive(Parser)]
#[command(version, about, name = "diaria")]
struct Cli {
    // Optional so that a bare `diaria` invocation defaults to `status` rather
    // than erroring out with "a subcommand is required".
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Add {
        #[arg(short = 'i', long)]
        input: Option<PathBuf>,
    },
    Read {
        filename: Option<PathBuf>,
    },
    Load {
        #[arg(short = 'd', long)]
        directory: PathBuf,
    },
    Dump {
        #[arg(short = 'd', long)]
        directory: Option<PathBuf>,
    },
    Sync,
    Summarize,
    Stats,
    Status,
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        // Print the error's `Display` (its human-facing message) rather than
        // the `Debug` form the default `Termination` impl would use.
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Status) {
        Commands::Init => Di::init().execute(),
        Commands::Add { input } => Di::add().execute(input.as_deref()),
        Commands::Read { filename } => Di::read().execute(filename.as_deref()),
        Commands::Load { directory } => Di::load().execute(&directory),
        Commands::Dump { directory } => Di::dump().execute(directory),
        Commands::Sync => Di::sync().execute(),
        Commands::Summarize => Di::summarize().execute(),
        Commands::Stats => Di::stats().execute(),
        Commands::Status => Di::status().execute(),
    }
}
