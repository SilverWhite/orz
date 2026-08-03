//! orz-config-types — minimal type stubs for kept Grok providers.
//! Contains only the types still referenced by orz-memory, orz-workspace, orz-shared.

use serde::{Deserialize, Serialize};

/// BoolFlag — a feature flag that can be enabled/disabled, possibly by remote config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoolFlag {
    pub value: bool,
}

impl BoolFlag {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
    pub fn is_enabled(&self) -> bool {
        self.value
    }
}

/// RemoteSettings — remote configuration settings stub.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteSettings {
    pub enabled: bool,
}

/// Memory embedding configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryEmbeddingConfig {
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
}

/// Memory search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchConfig {
    pub max_results: usize,
    pub similarity_threshold: f64,
}

impl Default for MemorySearchConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            similarity_threshold: 0.7,
        }
    }
}

/// Worktree auto-GC settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorktreeAutoGcSettings {
    pub enabled: bool,
    pub max_age_days: u32,
}

/// Worktree kind max age.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorktreeKindMaxAge {
    pub kind: String,
    pub max_age_days: u32,
}

/// Display refresh settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayRefreshSettings {
    pub interval_ms: u64,
}
