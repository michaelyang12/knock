use clap::{Parser, Subcommand};

/// Natural language to shell command translator
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,

    /// What you want to do (or search term with --history)
    #[arg(default_value = "")]
    pub(crate) input: String,

    /// Include explanation of the generated command
    #[arg(short, long)]
    pub(crate) verbose: bool,

    /// Show alternative commands and options
    #[arg(short, long)]
    pub(crate) alt: bool,

    /// Execute the command after confirmation
    #[arg(short = 'x', long)]
    pub(crate) execute: bool,

    /// Show command history (optionally filter by search term)
    #[arg(long)]
    pub(crate) history: bool,

    /// Configure provider and model
    #[arg(long)]
    pub(crate) config: bool,

    /// Upgrade to latest version from git
    #[arg(long)]
    pub(crate) upgrade: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Explain what a shell command does
    Explain {
        /// The command to explain
        command: String,
    },
}
