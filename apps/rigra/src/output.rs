//! Output rendering for lint, format, and sync commands.
//!
//! Supports `human` (default) and `json` outputs. The JSON form includes
//! per-item fields and a top-level summary.

use crate::models::LintResult;
use crate::{format::FormatResult, sync::SyncAction};
use owo_colors::OwoColorize;
use serde_json::json;

fn use_colors(output: &str) -> bool {
    output != "json" && std::env::var_os("NO_COLOR").is_none()
}

/// Print lint results in the requested format.
pub fn print_lint(res: &LintResult, output: &str) {
    match output {
        "json" => println!("{}", serde_json::to_string_pretty(res).unwrap()),
        _ => {
            let color = use_colors(output);
            for is in &res.issues {
                let sev = match is.severity.as_str() {
                    "error" => {
                        if color {
                            "[ERROR]".red().bold().to_string()
                        } else {
                            "[ERROR]".to_string()
                        }
                    }
                    "warning" | "warn" => {
                        if color {
                            "[WARN]".yellow().bold().to_string()
                        } else {
                            "[WARN]".to_string()
                        }
                    }
                    _ => {
                        if color {
                            "[INFO]".blue().bold().to_string()
                        } else {
                            "[INFO]".to_string()
                        }
                    }
                };
                let icon = match is.severity.as_str() {
                    "error" => "❌",
                    "warning" | "warn" => "⚠️",
                    _ => "ℹ️",
                };
                let file = if color {
                    is.file.clone().bold().to_string()
                } else {
                    is.file.clone()
                };
                println!(
                    "{} {} {} (rule={}) — {}",
                    icon, sev, file, is.rule, is.message
                );
            }
            let summary = format!(
                "— Summary — errors={} warnings={} infos={} files={}",
                res.summary.errors, res.summary.warnings, res.summary.infos, res.summary.files
            );
            if color {
                println!("{}", summary.bold());
            } else {
                println!("{}", summary);
            }
        }
    }
}

/// Print formatting results. When `write` is false, previews and diffs
/// can be emitted; otherwise only file statuses are shown.
pub fn print_format(results: &[FormatResult], output: &str, write: bool, diff: bool) {
    match output {
        "json" => {
            let items: Vec<_> = results
                .iter()
                .map(|r| {
                    json!({
                        "file": r.file,
                        "changed": r.changed,
                        "wrote": write && r.changed,
                        "preview": if !write { r.preview.as_ref() } else { None },
                        "diff": if diff && !write { build_naive_diff(r.original.as_deref(), r.preview.as_deref()) } else { None }
                    })
                })
                .collect();
            let summary = json!({
                "changed": results.iter().filter(|r| r.changed).count(),
                "total": results.len(),
                "wrote": if write { results.iter().filter(|r| r.changed).count() } else { 0 },
            });
            let out = json!({"results": items, "summary": summary});
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        _ => {
            let color = use_colors(output);
            for r in results {
                if write {
                    if r.changed {
                        if color {
                            println!("{} {}", "✏️  formatted:".green().bold(), r.file.bold());
                        } else {
                            println!("✏️  formatted: {}", r.file);
                        }
                    }
                } else if r.changed {
                    if diff {
                        if let Some(d) =
                            build_naive_diff(r.original.as_deref(), r.preview.as_deref())
                        {
                            if color {
                                println!("{} {}\n{}", "---".cyan().bold(), r.file.bold(), d);
                            } else {
                                println!("--- {}\n{}", r.file, d);
                            }
                        } else if let Some(prev) = &r.preview {
                            if color {
                                println!("{} {}\n{}", "---".cyan().bold(), r.file.bold(), prev);
                            } else {
                                println!("--- {}\n{}", r.file, prev);
                            }
                        }
                    } else if let Some(prev) = &r.preview {
                        if color {
                            println!("{} {}\n{}", "---".cyan().bold(), r.file.bold(), prev);
                        } else {
                            println!("--- {}\n{}", r.file, prev);
                        }
                    }
                } else {
                    if color {
                        println!("{} {}", "no changes:".bright_black().to_string(), r.file);
                    } else {
                        println!("no changes: {}", r.file);
                    }
                }
            }
        }
    }
}

/// Print sync actions summarizing writes and skips.
pub fn print_sync(actions: &[SyncAction], output: &str) {
    match output {
        "json" => {
            let items: Vec<_> = actions
                .iter()
                .map(|a| {
                    json!({
                        "rule": a.rule_id,
                        "source": a.source,
                        "target": a.target,
                        "wrote": a.wrote,
                        "skipped": a.skipped,
                    })
                })
                .collect();
            let summary = json!({
                "wrote": actions.iter().filter(|a| a.wrote).count(),
                "skipped": actions.iter().filter(|a| a.skipped).count(),
                "total": actions.len(),
            });
            let out = json!({"results": items, "summary": summary});
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        _ => {
            let color = use_colors(output);
            for a in actions {
                if a.skipped {
                    if color {
                        println!(
                            "{} {} -> {} (rule={})",
                            "⏭️  skipped (exists):".yellow().bold(),
                            a.source,
                            a.target,
                            a.rule_id
                        );
                    } else {
                        println!(
                            "⏭️  skipped (exists): {} -> {} (rule={})",
                            a.source, a.target, a.rule_id
                        );
                    }
                } else if a.wrote {
                    if color {
                        println!(
                            "{} {} -> {} (rule={})",
                            "📥 synced:".green().bold(),
                            a.source,
                            a.target,
                            a.rule_id
                        );
                    } else {
                        println!(
                            "📥 synced: {} -> {} (rule={})",
                            a.source, a.target, a.rule_id
                        );
                    }
                }
            }
        }
    }
}

fn build_naive_diff(old: Option<&str>, new: Option<&str>) -> Option<String> {
    let old = old?;
    let new = new?;
    let mut out = String::new();
    out.push_str("+++ new\n");
    out.push_str(new);
    out.push('\n');
    out.push_str("--- old\n");
    out.push_str(old);
    Some(out)
}
