//! CLI argument parsing via `clap`.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rigra", version, about = "Rigra v2 (Rust + TOML)")]
/// Top-level CLI options and subcommands.
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
/// Supported subcommands for linting, formatting, and syncing.
pub enum Commands {
    /// Show version
    Version,
    /// Lint configs using TOML policies
    Lint {
        #[arg(long)]
        repo_root: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        index: Option<String>,
    },
    /// Format files deterministically
    Format {
        #[arg(long)]
        repo_root: Option<String>,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        write: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        diff: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        check: bool,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        index: Option<String>,
    },
    /// Sync templates/configs
    Sync {
        #[arg(long)]
        repo_root: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        index: Option<String>,
    },
}
