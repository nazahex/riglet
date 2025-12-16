//! Utility helpers for paths and JSON navigation.

use serde_json::Value as Json;
use std::path::Path;

/// Return a path relative to the current working directory when possible.
pub fn rel_to_wd(p: &Path) -> String {
    match std::env::current_dir() {
        Ok(wd) => match pathdiff::diff_paths(p, wd) {
            Some(r) => r.to_string_lossy().to_string(),
            None => p.to_string_lossy().to_string(),
        },
        Err(_) => p.to_string_lossy().to_string(),
    }
}

/// Get nested value by a simple JSONPath-like string: `$.a.b.c` or `a.b.c`.
pub fn get_json_path<'a>(json: &'a Json, path: &str) -> Option<&'a Json> {
    let trimmed = path.trim();
    let p = if let Some(stripped) = trimmed.strip_prefix("$") {
        stripped.trim_start_matches('.')
    } else {
        trimmed
    };
    let mut cur = json;
    if p.is_empty() {
        return Some(cur);
    }
    for seg in p.split('.') {
        if seg.is_empty() {
            continue;
        }
        match cur {
            Json::Object(map) => {
                if let Some(v) = map.get(seg) {
                    cur = v;
                } else {
                    return None;
                }
            }
            _ => {
                return None;
            }
        }
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_json_path_basic_and_nested() {
        let data = serde_json::json!({
            "name": "rigra",
            "nested": { "a": { "b": 42 } }
        });
        assert_eq!(
            get_json_path(&data, "name").unwrap(),
            &Json::String("rigra".into())
        );
        assert_eq!(
            get_json_path(&data, "$.nested.a.b").unwrap(),
            &Json::from(42)
        );
        assert!(get_json_path(&data, "nested.missing").is_none());
        assert!(get_json_path(&data, "$.nested.a.b.c").is_none());
    }
}
