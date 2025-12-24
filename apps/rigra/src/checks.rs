//! Implementation of policy-driven validation checks.
//!
//! Supported check kinds: `required`, `type`, `const`, `pattern`, `enum`,
//! `minLength`, `maxLength`. Paths accept a simple `$.a.b` or `a.b` syntax.

use crate::models::policy::Check;
use crate::models::Issue;
use crate::utils::{get_json_path, rel_to_wd};
use regex::Regex;
use serde_json::Value as Json;
use std::collections::HashMap;
use std::path::PathBuf;

/// Execute all checks against a JSON value, producing `Issue`s.
pub fn run_checks(checks: &[Check], json: &Json, path: &PathBuf, rule_id: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    // Cache compiled regex per unique pattern to avoid recompilation within a run
    let mut re_cache: HashMap<String, Regex> = HashMap::new();
    for chk in checks.iter().cloned() {
        match chk {
            Check::Required {
                fields,
                message,
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                for f in fields {
                    let missing = get_json_path(json, &f).is_none();
                    if missing {
                        let norm = f.trim_start_matches('$').trim_start_matches('.');
                        let msg = message
                            .clone()
                            .unwrap_or_else(|| {
                                "Field '{{field}}' is required at $.{{field}}".to_string()
                            })
                            .replace("{{field}}", norm)
                            .replace("{{path}}", &format!("$.{}", norm));
                        issues.push(Issue {
                            file: rel_to_wd(path),
                            rule: rule_id.to_string(),
                            severity: sev.clone(),
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
                fields,
                message,
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                let base = message
                    .clone()
                    .unwrap_or_else(|| "Expected {{kind}} at $.{{path}}".to_string());

                // Recommended path->kind checks
                for (p, kind) in fields.iter() {
                    if let Some(v) = get_json_path(json, p) {
                        if !is_type(v, kind) {
                            let norm = p.trim_start_matches('$').trim_start_matches('.');
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: sev.clone(),
                                path: format!("$.{}", norm),
                                message: base
                                    .replace("{{kind}}", kind)
                                    .replace("{{path}}", &format!("$.{}", norm))
                                    .replace("{{actual}}", json_kind(v)),
                            });
                        }
                    }
                }
            }
            Check::Const {
                field,
                value,
                message,
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                let got = get_json_path(json, &field);
                if got != Some(&value) {
                    let norm = field.trim_start_matches('$').trim_start_matches('.');
                    let msg = message
                        .clone()
                        .unwrap_or_else(|| "Field must equal expected value".to_string())
                        .replace("{{expected}}", &value.to_string())
                        .replace(
                            "{{actual}}",
                            &got.map(|g| g.to_string())
                                .unwrap_or_else(|| "null".to_string()),
                        )
                        .replace("{{path}}", &format!("$.{}", norm));
                    issues.push(Issue {
                        file: rel_to_wd(path),
                        rule: rule_id.to_string(),
                        severity: sev,
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
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                if let Some(v) = get_json_path(json, &field) {
                    if let Some(s) = v.as_str() {
                        let re = re_cache.entry(regex.clone()).or_insert_with(|| {
                            Regex::new(&regex).unwrap_or_else(|_| Regex::new("^$").unwrap())
                        });
                        if !re.is_match(s) {
                            let norm = field.trim_start_matches('$').trim_start_matches('.');
                            let msg = message
                                .clone()
                                .unwrap_or_else(|| "Pattern mismatch".to_string())
                                .replace("{{pattern}}", &regex)
                                .replace("{{actual}}", s)
                                .replace("{{path}}", &format!("$.{}", norm));
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: sev,
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
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                if let Some(actual) = get_json_path(json, &field) {
                    if !values.iter().any(|v| v == actual) {
                        let norm = field.trim_start_matches('$').trim_start_matches('.');
                        let msg = message
                            .clone()
                            .unwrap_or_else(|| "Value not in allowed set".to_string())
                            .replace("{{expected}}", &format!("{:?}", values))
                            .replace("{{actual}}", &actual.to_string())
                            .replace("{{path}}", &format!("$.{}", norm));
                        issues.push(Issue {
                            file: rel_to_wd(path),
                            rule: rule_id.to_string(),
                            severity: sev,
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
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                if let Some(v) = get_json_path(json, &field) {
                    if let Some(s) = v.as_str() {
                        if s.len() < min {
                            let msg = message
                                .clone()
                                .unwrap_or_else(|| "String shorter than minimum".to_string())
                                .replace("{{expected}}", &min.to_string())
                                .replace("{{actual}}", &s.len().to_string())
                                .replace(
                                    "{{path}}",
                                    &format!(
                                        "$.{}",
                                        field.trim_start_matches('$').trim_start_matches('.')
                                    ),
                                );
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: sev,
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
                level,
            } => {
                let sev = level.unwrap_or_else(|| "error".to_string());
                if let Some(v) = get_json_path(json, &field) {
                    if let Some(s) = v.as_str() {
                        if s.len() > max {
                            let msg = message
                                .clone()
                                .unwrap_or_else(|| "String longer than maximum".to_string())
                                .replace("{{expected}}", &max.to_string())
                                .replace("{{actual}}", &s.len().to_string())
                                .replace(
                                    "{{path}}",
                                    &format!(
                                        "$.{}",
                                        field.trim_start_matches('$').trim_start_matches('.')
                                    ),
                                );
                            issues.push(Issue {
                                file: rel_to_wd(path),
                                rule: rule_id.to_string(),
                                severity: sev,
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

fn is_type(v: &Json, kind: &str) -> bool {
    match kind {
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.as_i64().is_some(),
        "boolean" => v.is_boolean(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        "null" => v.is_null(),
        _ => false,
    }
}

fn json_kind(v: &Json) -> &'static str {
    if v.is_string() {
        "string"
    } else if v.is_boolean() {
        "boolean"
    } else if v.is_array() {
        "array"
    } else if v.is_object() {
        "object"
    } else if v.is_null() {
        "null"
    } else if v.as_i64().is_some() {
        "integer"
    } else if v.is_number() {
        "number"
    } else {
        "unknown"
    }
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
                level: None,
            },
            Check::Type {
                fields: vec![
                    ("name".into(), "string".into()),
                    ("version".into(), "string".into()),
                ]
                .into_iter()
                .collect(),
                message: None,
                level: None,
            },
            Check::Const {
                field: "version".into(),
                value: json!("2.0.0"),
                message: None,
                level: None,
            },
            Check::Pattern {
                field: "nested.x".into(),
                regex: "^xyz$".into(),
                message: None,
                level: None,
            },
            Check::Enum {
                field: "choice".into(),
                values: vec![json!("alpha"), json!("beta")],
                message: None,
                level: None,
            },
            Check::MinLength {
                field: "short".into(),
                min: 2,
                message: None,
                level: None,
            },
            Check::MaxLength {
                field: "long".into(),
                max: 5,
                message: None,
                level: None,
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

    #[test]
    fn test_type_fields_all_kinds_match() {
        let json = json!({
            "s": "str",
            "n": 1.5,
            "i": 2,
            "b": true,
            "a": [1,2,3],
            "o": {"k":"v"},
            "z": null
        });
        let path = PathBuf::from("file.json");
        let mut fields = HashMap::new();
        fields.insert("s".into(), "string".into());
        fields.insert("n".into(), "number".into());
        fields.insert("i".into(), "integer".into());
        fields.insert("b".into(), "boolean".into());
        fields.insert("a".into(), "array".into());
        fields.insert("o".into(), "object".into());
        fields.insert("z".into(), "null".into());
        let checks = vec![Check::Type {
            fields,
            message: None,
            level: None,
        }];
        let issues = run_checks(&checks, &json, &path, "rule");
        assert!(issues.is_empty());
    }

    #[test]
    fn test_type_fields_all_kinds_mismatch() {
        let json = json!({
            "s": 10,
            "n": "not-number",
            "i": 1.5,
            "b": "true",
            "a": {"not":"array"},
            "o": [1,2,3],
            "z": "not-null"
        });
        let path = PathBuf::from("file.json");
        let mut fields = HashMap::new();
        fields.insert("s".into(), "string".into());
        fields.insert("n".into(), "number".into());
        fields.insert("i".into(), "integer".into());
        fields.insert("b".into(), "boolean".into());
        fields.insert("a".into(), "array".into());
        fields.insert("o".into(), "object".into());
        fields.insert("z".into(), "null".into());
        let checks = vec![Check::Type {
            fields,
            message: Some("Type mismatch at {{path}}, expected {{kind}}, got {{actual}}".into()),
            level: None,
        }];
        let issues = run_checks(&checks, &json, &path, "rule");
        // Expect 7 issues, one per path
        assert_eq!(issues.len(), 7);
        let paths: std::collections::HashSet<_> = issues.iter().map(|i| i.path.clone()).collect();
        for p in ["$.s", "$.n", "$.i", "$.b", "$.a", "$.o", "$.z"].iter() {
            assert!(paths.contains(&p.to_string()));
        }
        // spot-check a couple of messages include actual kind names
        let msg_s = issues
            .iter()
            .find(|i| i.path == "$.s")
            .unwrap()
            .message
            .clone();
        assert!(msg_s.contains("got integer"));
        let msg_a = issues
            .iter()
            .find(|i| i.path == "$.a")
            .unwrap()
            .message
            .clone();
        assert!(msg_a.contains("got object"));
    }

    #[test]
    fn test_required_only_missing_reported() {
        let json = json!({"a":1, "b":2});
        let path = PathBuf::from("file.json");
        let checks = vec![Check::Required {
            fields: vec!["a".into(), "c".into()],
            message: None,
            level: None,
        }];
        let issues = run_checks(&checks, &json, &path, "rule");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$.c");
    }

    #[test]
    fn test_const_match_and_mismatch() {
        let json = json!({"x":"y", "n": 3});
        let path = PathBuf::from("file.json");
        let checks = vec![
            Check::Const {
                field: "x".into(),
                value: json!("y"),
                message: Some("Field at {{path}} must equal {{expected}}, got {{actual}}".into()),
                level: None,
            },
            Check::Const {
                field: "n".into(),
                value: json!(4),
                message: Some("Field at {{path}} must equal {{expected}}, got {{actual}}".into()),
                level: None,
            },
        ];
        let issues = run_checks(&checks, &json, &path, "rule");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$.n");
        // Message interpolation includes expected, actual, and path
        assert!(issues[0].message.contains("must equal 4"));
        assert!(issues[0].message.contains("got 3") || issues[0].message.contains("3"));
        assert!(issues[0].message.contains("$.n"));
    }

    #[test]
    fn test_pattern_match_and_mismatch() {
        let json = json!({"v":"1.2.3", "w":"nope"});
        let path = PathBuf::from("file.json");
        let checks = vec![
            Check::Pattern {
                field: "v".into(),
                regex: "^\\d+\\.\\d+\\.\\d+$".into(),
                message: Some("Value '{{actual}}' at {{path}} must match {{pattern}}".into()),
                level: None,
            },
            Check::Pattern {
                field: "w".into(),
                regex: "^\\d+$".into(),
                message: Some("Value '{{actual}}' at {{path}} must match {{pattern}}".into()),
                level: None,
            },
        ];
        let issues = run_checks(&checks, &json, &path, "rule");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$.w");
        assert_eq!(issues[0].message, "Value 'nope' at $.w must match ^\\d+$");
    }

    #[test]
    fn test_enum_match_and_mismatch() {
        let json = json!({"k":"b", "n": 2});
        let path = PathBuf::from("file.json");
        let checks = vec![
            Check::Enum {
                field: "k".into(),
                values: vec![json!("a"), json!("b")],
                message: Some("Value at {{path}} must be one of {{expected}}, got {{actual}}".into()),
                level: None,
            },
            Check::Enum {
                field: "n".into(),
                values: vec![json!(1), json!(3)],
                message: Some("Value at {{path}} must be one of {{expected}}, got {{actual}}".into()),
                level: None,
            },
        ];
        let issues = run_checks(&checks, &json, &path, "rule");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$.n");
        // Message interpolation includes expected set, actual value, and path
        assert!(issues[0].message.contains("one of"));
        assert!(issues[0].message.contains("2"));
        assert!(issues[0].message.contains("$.n"));
    }

    #[test]
    fn test_min_max_length_boundaries() {
        let json = json!({"s1":"ab", "s2":"a", "s3":"abc", "s4":"abcdef"});
        let path = PathBuf::from("file.json");
        let checks = vec![
            Check::MinLength {
                field: "s1".into(),
                min: 2,
                message: Some("String at {{path}} length must be >= {{expected}}, got {{actual}}".into()),
                level: None,
            }, // ok
            Check::MinLength {
                field: "s2".into(),
                min: 2,
                message: Some("String at {{path}} length must be >= {{expected}}, got {{actual}}".into()),
                level: None,
            }, // fail
            Check::MaxLength {
                field: "s3".into(),
                max: 3,
                message: Some("String at {{path}} length must be <= {{expected}}, got {{actual}}".into()),
                level: None,
            }, // ok
            Check::MaxLength {
                field: "s4".into(),
                max: 5,
                message: Some("String at {{path}} length must be <= {{expected}}, got {{actual}}".into()),
                level: None,
            }, // fail
        ];
        let issues = run_checks(&checks, &json, &path, "rule");
        let paths: std::collections::HashSet<_> = issues.iter().map(|i| i.path.clone()).collect();
        assert_eq!(issues.len(), 2);
        assert!(paths.contains("$.s2"));
        assert!(paths.contains("$.s4"));
        // Message interpolation includes expected, actual, and path in both issues
        let m2 = issues.iter().find(|i| i.path == "$.s2").unwrap().message.clone();
        assert!(m2.contains("$.s2"));
        assert!(m2.contains(">= 2"));
        let m4 = issues.iter().find(|i| i.path == "$.s4").unwrap().message.clone();
        assert!(m4.contains("$.s4"));
        assert!(m4.contains("<= 5"));
    }

    #[test]
    fn test_required_message_interpolation_path() {
        let json = json!({"a":1});
        let path = PathBuf::from("file.json");
        let checks = vec![Check::Required { fields: vec!["a".into(), "b".into()], message: Some("Field '{{field}}' missing at {{path}}".into()), level: None }];
        let issues = run_checks(&checks, &json, &path, "rule");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$.b");
        assert_eq!(issues[0].message, "Field 'b' missing at $.b");
    }
}
