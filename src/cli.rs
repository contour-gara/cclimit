use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Print Claude Code usage limits in one line")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Print diagnostics for credentials and API")]
    Doctor,
}
