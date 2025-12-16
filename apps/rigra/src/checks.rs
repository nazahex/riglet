//! Implementation of policy-driven validation checks.
//!
//! Supported check kinds: `required`, `type`, `const`, `pattern`, `enum`,
//! `minLength`, `maxLength`. Paths accept a simple `$.a.b` or `a.b` syntax.

use crate::models::policy::Check;
use crate::models::Issue;
use crate::utils::{get_json_path, rel_to_wd};
use regex::Regex;
use serde_json::Value as Json;
use std::path::PathBuf;

/// Execute all checks against a JSON value, producing `Issue`s.
pub fn run_checks(checks: &[Check], json: &Json, path: &PathBuf, rule_id: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    for chk in checks.iter().cloned() {
        match chk {
            Check::Required { fields, message } => {
                for f in fields {
                    let missing = get_json_path(json, &f).is_none();
                    if missing {
                        let msg = message
                            .clone()
                            .unwrap_or_else(|| {
                                "Field '{{field}}' is required at $.{{field}}".to_string()
                            })
                            .replace(
                                "{{field}}",
                                &f.trim_start_matches('$').trim_start_matches('.'),
                            );
                        issues.push(Issue {
                            file: rel_to_wd(path),
                            rule: rule_id.to_string(),
                            severity: "error".into(),
                            path: format!(
                                "$.{}",
                                f.trim_start_matches('$').trim_start_matches('.')
                            ),
                            message: msg,
                        });
                    }
                }
            }
            Check::Type {
                name,
                version,
                message,
            } => {
                let base = message
                    .clone()
                    .unwrap_or_else(|| "Expected string at $.{{path}}".to_string());
                if name.is_some() {
                    if let Some(v) = get_json_path(json, "name") {
                        if !v.is_string() {
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: "error".into(),
                                path: "$.name".into(),
                                message: base.replace("{{path}}", "name"),
                            });
                        }
                    }
                }
                if version.is_some() {
                    if let Some(v) = get_json_path(json, "version") {
                        if !v.is_string() {
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: "error".into(),
                                path: "$.version".into(),
                                message: base.replace("{{path}}", "version"),
                            });
                        }
                    }
                }
            }
            Check::Const {
                field,
                value,
                message,
            } => {
                let got = get_json_path(json, &field);
                if got != Some(&value) {
                    let msg = message
                        .clone()
                        .unwrap_or_else(|| "Field must equal expected value".to_string())
                        .replace("{{expected}}", &value.to_string());
                    issues.push(Issue {
                        file: rel_to_wd(path),
                        rule: rule_id.to_string(),
                        severity: "error".into(),
                        path: format!(
                            "$.{}",
                            field.trim_start_matches('$').trim_start_matches('.')
                        ),
                        message: msg,
                    });
                }
            }
            Check::Pattern {
                field,
                regex,
                message,
            } => {
                if let Some(v) = get_json_path(json, &field) {
                    if let Some(s) = v.as_str() {
                        let re = Regex::new(&regex).unwrap_or_else(|_| Regex::new("^$").unwrap());
                        if !re.is_match(s) {
                            let msg = message
                                .clone()
                                .unwrap_or_else(|| "Pattern mismatch".to_string());
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: "error".into(),
                                path: format!(
                                    "$.{}",
                                    field.trim_start_matches('$').trim_start_matches('.')
                                ),
                                message: msg,
                            });
                        }
                    }
                }
            }
            Check::Enum {
                field,
                values,
                message,
            } => {
                if let Some(actual) = get_json_path(json, &field) {
                    if !values.iter().any(|v| v == actual) {
                        let msg = message
                            .clone()
                            .unwrap_or_else(|| "Value not in allowed set".to_string())
                            .replace("{{expected}}", &format!("{:?}", values))
                            .replace("{{actual}}", &actual.to_string());
                        issues.push(Issue {
                            file: rel_to_wd(path),
                            rule: rule_id.to_string(),
                            severity: "error".into(),
                            path: format!(
                                "$.{}",
                                field.trim_start_matches('$').trim_start_matches('.')
                            ),
                            message: msg,
                        });
                    }
                }
            }
            Check::MinLength {
                field,
                min,
                message,
            } => {
                if let Some(v) = get_json_path(json, &field) {
                    if let Some(s) = v.as_str() {
                        if s.len() < min {
                            let msg = message
                                .clone()
                                .unwrap_or_else(|| "String shorter than minimum".to_string())
                                .replace("{{expected}}", &min.to_string())
                                .replace("{{actual}}", &s.len().to_string());
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: "error".into(),
                                path: format!(
                                    "$.{}",
                                    field.trim_start_matches('$').trim_start_matches('.')
                                ),
                                message: msg,
                            });
                        }
                    }
                }
            }
            Check::MaxLength {
                field,
                max,
                message,
            } => {
                if let Some(v) = get_json_path(json, &field) {
                    if let Some(s) = v.as_str() {
                        if s.len() > max {
                            let msg = message
                                .clone()
                                .unwrap_or_else(|| "String longer than maximum".to_string())
                                .replace("{{expected}}", &max.to_string())
                                .replace("{{actual}}", &s.len().to_string());
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: "error".into(),
                                path: format!(
                                    "$.{}",
                                    field.trim_start_matches('$').trim_start_matches('.')
                                ),
                                message: msg,
                            });
                        }
                    }
                }
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_run_checks_various_and_nested() {
        let json = json!({
            "name": 123,
            "version": "1.0.0",
            "nested": { "x": "abc" },
            "choice": "gamma",
            "short": "a",
            "long": "abcdef"
        });
        let path = PathBuf::from("package.json");
        let checks = vec![
            Check::Required {
                fields: vec!["nested.x".into(), "missing.field".into()],
                message: None,
            },
            Check::Type {
                name: Some("string".into()),
                version: Some("string".into()),
                message: None,
            },
            Check::Const {
                field: "version".into(),
                value: json!("2.0.0"),
                message: None,
            },
            Check::Pattern {
                field: "nested.x".into(),
                regex: "^xyz$".into(),
                message: None,
            },
            Check::Enum {
                field: "choice".into(),
                values: vec![json!("alpha"), json!("beta")],
                message: None,
            },
            Check::MinLength {
                field: "short".into(),
                min: 2,
                message: None,
            },
            Check::MaxLength {
                field: "long".into(),
                max: 5,
                message: None,
            },
        ];
        let issues = run_checks(&checks, &json, &path, "t");
        // Expect errors for: required(missing.field), type(name not string), const(version), pattern(nested.x), enum(choice), minLength(short), maxLength(long)
        assert!(issues.iter().any(|i| i.path == "$.missing.field"));
        assert!(issues.iter().any(|i| i.path == "$.name"));
        assert!(issues.iter().any(|i| i.path == "$.version"));
        assert!(issues.iter().any(|i| i.path == "$.nested.x"));
        assert!(issues.iter().any(|i| i.path == "$.choice"));
        assert!(issues.iter().any(|i| i.path == "$.short"));
        assert!(issues.iter().any(|i| i.path == "$.long"));
    }
}
