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
        #[arg(long, action = clap::ArgAction::SetTrue)]
        write: bool,
        #[arg(long, action = clap::ArgAction::SetTrue, help = "Preview planned writes without changing files")]
        dry_run: bool,
        #[arg(long, action = clap::ArgAction::SetTrue, help = "Exit non-zero if changes would occur")]
        check: bool,
    },
    /// Convention management (install/list/prune/path)
    Conv {
        #[command(subcommand)]
        cmd: ConvCmd,
    },
}

#[derive(Subcommand)]
/// Subcommands for `rigra conv`
pub enum ConvCmd {
    /// Install a convention into cache
    Install {
        #[arg(long)]
        repo_root: Option<String>,
        /// Optional source override: gh:owner/repo@tag or file:/abs/path
        source: Option<String>,
        /// Optional name@version override for cache key
        #[arg(long)]
        name: Option<String>,
    },
    /// List installed conventions
    Ls {
        #[arg(long)]
        repo_root: Option<String>,
    },
    /// Prune all convention cache
    Prune {
        #[arg(long)]
        repo_root: Option<String>,
    },
    /// Resolve a conv path (conv:name@ver[:subpath])
    Path {
        #[arg(long)]
        repo_root: Option<String>,
        conv: String,
    },
}
