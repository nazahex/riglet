//! Rigra CLI binary entry point.
//! Delegates to modules for lint/format/sync and prints results.

mod checks;
mod cli;
mod config;
mod format;
mod lint;
mod models;
mod output;
mod sync;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use owo_colors::OwoColorize;

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Lint {
            repo_root,
            scope,
            output,
            index,
        } => {
            let eff = config::resolve_effective(
                repo_root.as_deref(),
                index.as_deref(),
                scope.as_deref(),
                output.as_deref(),
                None,
                None,
                None,
            );
            // Friendly note if no rigra config was found
            if config::load_config(&eff.repo_root).is_none() {
                eprintln!(
                    "{} {}",
                    "ℹ️  note:".blue().bold(),
                    "No rigra.{toml,yaml} found; using defaults."
                );
            }
            // Friendly error if index file is missing
            let idx_path = eff.repo_root.join(&eff.index);
            if !idx_path.exists() {
                eprintln!(
                    "{} {}",
                    "❌ error:".red().bold(),
                    format!(
                        "Index file not found: {} (pass --index or configure rigra.toml)",
                        idx_path.to_string_lossy()
                    )
                );
                std::process::exit(2);
            }
            let result = lint::run_lint(eff.repo_root.to_str().unwrap(), &eff.index);
            output::print_lint(&result, &eff.output);
            if result.summary.errors > 0 {
                std::process::exit(1);
            }
        }
        Commands::Format {
            repo_root,
            write,
            diff,
            check,
            output,
            index,
        } => {
            let eff = config::resolve_effective(
                repo_root.as_deref(),
                index.as_deref(),
                None,
                output.as_deref(),
                Some(write),
                Some(diff),
                Some(check),
            );
            if config::load_config(&eff.repo_root).is_none() {
                eprintln!(
                    "{} {}",
                    "ℹ️  note:".blue().bold(),
                    "No rigra.{toml,yaml} found; using defaults."
                );
            }
            let idx_path = eff.repo_root.join(&eff.index);
            if !idx_path.exists() {
                eprintln!(
                    "{} {}",
                    "❌ error:".red().bold(),
                    format!(
                        "Index file not found: {} (pass --index or configure rigra.toml)",
                        idx_path.to_string_lossy()
                    )
                );
                std::process::exit(2);
            }
            let results = format::run_format(
                eff.repo_root.to_str().unwrap(),
                &eff.index,
                eff.write,
                eff.diff || eff.check,
                eff.strict_linebreak,
                eff.lb_between_groups,
                &eff.lb_before_fields,
                &eff.lb_in_fields,
            );
            output::print_format(&results, &eff.output, eff.write, eff.diff);
            if eff.check && results.iter().any(|r| r.changed) {
                std::process::exit(1);
            }
        }
        Commands::Sync {
            repo_root,
            scope,
            output,
            index,
        } => {
            let eff = config::resolve_effective(
                repo_root.as_deref(),
                index.as_deref(),
                scope.as_deref(),
                output.as_deref(),
                None,
                None,
                None,
            );
            if config::load_config(&eff.repo_root).is_none() {
                eprintln!(
                    "{} {}",
                    "ℹ️  note:".blue().bold(),
                    "No rigra.{toml,yaml} found; using defaults."
                );
            }
            let idx_path = eff.repo_root.join(&eff.index);
            if !idx_path.exists() {
                eprintln!(
                    "{} {}",
                    "❌ error:".red().bold(),
                    format!(
                        "Index file not found: {} (pass --index or configure rigra.toml)",
                        idx_path.to_string_lossy()
                    )
                );
                std::process::exit(2);
            }
            let actions = sync::run_sync(eff.repo_root.to_str().unwrap(), &eff.index, &eff.scope);
            output::print_sync(&actions, &eff.output);
        }
    }
}
