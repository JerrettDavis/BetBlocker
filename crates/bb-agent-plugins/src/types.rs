use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration passed to a plugin during init.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin-specific key-value settings.
    pub settings: HashMap<String, serde_json::Value>,
    /// Whether this plugin is enabled (can be toggled by enrollment policy).
    pub enabled: bool,
    /// Priority relative to other plugins in the same layer (lower = checked first).
    pub priority: u32,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            settings: HashMap::new(),
            enabled: true,
            priority: 100,
        }
    }
}

/// Health status returned by plugin health checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub message: String,
    pub checked_at: DateTime<Utc>,
    /// Optional details for diagnostics (not sent to API).
    pub details: HashMap<String, String>,
}

impl PluginHealth {
    pub fn ok() -> Self {
        Self {
            healthy: true,
            message: "OK".into(),
            checked_at: Utc::now(),
            details: HashMap::new(),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: message.into(),
            checked_at: Utc::now(),
            details: HashMap::new(),
        }
    }
}

/// The blocking layer a plugin belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockingLayer {
    Dns,
    App,
    Browser,
}

/// Result of a blocking check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockDecision {
    /// Domain/app is not in the blocklist -- allow through.
    Allow,
    /// Domain/app is blocked. `reason` is a human-readable string for logging.
    Block { reason: String },
    /// Plugin cannot determine -- defer to next plugin.
    Abstain,
}

impl BlockDecision {
    pub fn is_blocked(&self) -> bool {
        matches!(self, BlockDecision::Block { .. })
    }
}

impl fmt::Display for BlockDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockDecision::Allow => write!(f, "Allow"),
            BlockDecision::Block { reason } => write!(f, "Block({reason})"),
            BlockDecision::Abstain => write!(f, "Abstain"),
        }
    }
}

/// Errors returned by plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("Activation failed: {0}")]
    ActivationFailed(String),

    #[error("Plugin is not healthy: {0}")]
    Unhealthy(String),

    #[error("OS prerequisite missing: {0}")]
    PrerequisiteMissing(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Identifier for an application (used by `AppBlockingPlugin`).
/// Placeholder for Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIdentifier {
    pub package_name: Option<String>,
    pub executable_path: Option<String>,
    pub display_name: Option<String>,
}

/// Match result from app scanning. Placeholder for Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMatch {
    pub app_id: AppIdentifier,
    pub confidence: f64,
    pub reason: String,
}

/// Content blocking rules for browser extensions. Placeholder for Phase 3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRules {
    pub rules_json: String,
    pub generated_at: DateTime<Utc>,
}

/// Browser extension health. Placeholder for Phase 3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionHealth {
    pub installed: bool,
    pub version: Option<String>,
    pub integrity_ok: bool,
}
