//! Sync policy file schema: defaults + per-id rules.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SyncPolicy {
    #[serde(default)]
    pub lint: Option<SyncLintDefaults>,
    #[serde(default)]
    pub sync: Vec<SyncRule>,
}

#[derive(Deserialize, Default)]
pub struct SyncLintDefaults {
    pub level: Option<String>,
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct SyncRule {
    pub id: String,
    pub source: String,
    pub target: String,
    pub when: String,
    /// Optional format type for structured files: json|yaml|toml
    #[serde(default)]
    pub format: Option<String>,
    /// Optional lint overrides for this rule
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}
