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

use crate::models::index::Index;
use clap::Parser;
use cli::{Cli, Commands};
use owo_colors::OwoColorize;
use std::fs;

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
            // Require index to be configured (no default)
            if !eff.index_configured {
                eprintln!(
                    "{} {}",
                    "❌ error:".red().bold(),
                    "Index is not configured. Pass --index or add rigra.{toml,yaml}."
                );
                std::process::exit(2);
            }
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
            // Emit single top info when default patterns from index are used (no overrides in rigra.toml)
            if eff.output != "json" {
                if let Ok(s) = fs::read_to_string(&idx_path) {
                    if let Ok(ix) = toml::from_str::<Index>(&s) {
                        let mut pat_set: std::collections::BTreeSet<String> =
                            std::collections::BTreeSet::new();
                        for r in ix.rules.iter() {
                            if !eff.pattern_overrides.contains_key(&r.id) {
                                for p in r.patterns.iter() {
                                    pat_set.insert(p.clone());
                                }
                            }
                        }
                        if !pat_set.is_empty() {
                            let joined =
                                format!("[{}]", pat_set.into_iter().collect::<Vec<_>>().join(", "));
                            eprintln!(
                                "{} {}",
                                "◆ ⟦info⟧".blue().bold(),
                                format!("Using default patterns: {}", joined)
                            );
                        }
                    }
                }
            }
            let result = lint::run_lint(
                eff.repo_root.to_str().unwrap(),
                &eff.index,
                &eff.pattern_overrides,
            );
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
                if write { Some(true) } else { None },
                if diff { Some(true) } else { None },
                if check { Some(true) } else { None },
            );
            if !eff.index_configured {
                eprintln!(
                    "{} {}",
                    "❌ error:".red().bold(),
                    "Index is not configured. Pass --index or add rigra.{toml,yaml}."
                );
                std::process::exit(2);
            }
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
            // Emit single top info when default patterns from index are used (no overrides in rigra.toml)
            if eff.output != "json" {
                if let Ok(s) = fs::read_to_string(&idx_path) {
                    if let Ok(ix) = toml::from_str::<Index>(&s) {
                        let mut pat_set: std::collections::BTreeSet<String> =
                            std::collections::BTreeSet::new();
                        for r in ix.rules.iter() {
                            if !eff.pattern_overrides.contains_key(&r.id) {
                                for p in r.patterns.iter() {
                                    pat_set.insert(p.clone());
                                }
                            }
                        }
                        if !pat_set.is_empty() {
                            let joined =
                                format!("[{}]", pat_set.into_iter().collect::<Vec<_>>().join(", "));
                            eprintln!(
                                "{} {}",
                                "◆ ⟦info⟧".blue().bold(),
                                format!("Using default patterns: {}", joined)
                            );
                        }
                    }
                }
            }
            // CLI/config precedence at runtime:
            // - If diff or check is enabled, force write=false for this run.
            // - Otherwise respect write.
            let eff_diff = eff.diff;
            let eff_check = eff.check;
            let eff_write = if eff_diff || eff_check {
                false
            } else {
                eff.write
            };
            let results = format::run_format(
                eff.repo_root.to_str().unwrap(),
                &eff.index,
                eff_write,
                eff_diff || eff_check,
                eff.strict_linebreak,
                eff.lb_between_groups,
                &eff.lb_before_fields,
                &eff.lb_in_fields,
                &eff.pattern_overrides,
            );
            output::print_format(&results, &eff.output, eff_write, eff_diff);
            if eff_check && results.iter().any(|r| r.changed) {
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
