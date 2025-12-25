//! Utility helpers for paths and JSON navigation.

use owo_colors::OwoColorize;
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

/// Whether colors should be used for global messages (checks NO_COLOR).
pub fn use_colors_global() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

/// Standardized error prefix for human-readable output.
/// Returns colored "✖ ⟦error⟧" when colors are enabled, plain otherwise.
pub fn error_prefix() -> String {
    if use_colors_global() {
        "✖ ⟦error⟧".red().bold().to_string()
    } else {
        "✖ ⟦error⟧".to_string()
    }
}

/// Standardized info prefix for human-readable output.
pub fn info_prefix() -> String {
    if use_colors_global() {
        "◆ ⟦info⟧".blue().bold().to_string()
    } else {
        "◆ ⟦info⟧".to_string()
    }
}

/// Standardized note prefix for human-readable output.
pub fn note_prefix() -> String {
    if use_colors_global() {
        "◆ ⟦note⟧".blue().bold().to_string()
    } else {
        "◆ ⟦note⟧".to_string()
    }
}

/// Standardized warn prefix for human-readable output.
#[allow(dead_code)]
pub fn warn_prefix() -> String {
    if use_colors_global() {
        "▲ ⟦warn⟧".yellow().bold().to_string()
    } else {
        "▲ ⟦warn⟧".to_string()
    }
}

/// Colored severity tags without icons, controlled by caller-provided color flag.
pub fn tag_error(use_color: bool) -> String {
    if use_color {
        "⟦error⟧".red().bold().to_string()
    } else {
        "⟦error⟧".to_string()
    }
}

pub fn tag_warn(use_color: bool) -> String {
    if use_color {
        "⟦warn⟧".yellow().bold().to_string()
    } else {
        "⟦warn⟧".to_string()
    }
}

pub fn tag_info(use_color: bool) -> String {
    if use_color {
        "⟦info⟧".blue().bold().to_string()
    } else {
        "⟦info⟧".to_string()
    }
}

/// Colored icons for severity levels, controlled by caller-provided color flag.
pub fn icon_error(use_color: bool) -> String {
    if use_color {
        "✖".red().to_string()
    } else {
        "✖".to_string()
    }
}

pub fn icon_warn(use_color: bool) -> String {
    if use_color {
        "▲".yellow().to_string()
    } else {
        "▲".to_string()
    }
}

pub fn icon_info(use_color: bool) -> String {
    if use_color {
        "◆".blue().to_string()
    } else {
        "◆".to_string()
    }
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
