use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bb_agent_plugins::types::PluginConfig;
use bb_common::models::Enrollment;
use serde::{Deserialize, Serialize};

/// DNS-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// Upstream DNS servers to forward non-blocked queries to.
    #[serde(default = "DnsConfig::default_upstream_servers")]
    pub upstream_servers: Vec<String>,
    /// Address to listen on for DNS queries.
    #[serde(default = "DnsConfig::default_listen_addr")]
    pub listen_addr: String,
    /// Port to listen on for DNS queries.
    #[serde(default = "DnsConfig::default_listen_port")]
    pub listen_port: u16,
}

impl DnsConfig {
    fn default_upstream_servers() -> Vec<String> {
        vec!["8.8.8.8:53".into(), "1.1.1.1:53".into()]
    }

    fn default_listen_addr() -> String {
        "127.0.0.1".into()
    }

    fn default_listen_port() -> u16 {
        53
    }
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            upstream_servers: Self::default_upstream_servers(),
            listen_addr: Self::default_listen_addr(),
            listen_port: Self::default_listen_port(),
        }
    }
}

/// Reporting settings for the event system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingSettings {
    /// How often to flush events to the API (seconds).
    #[serde(default = "ReportingSettings::default_flush_interval")]
    pub flush_interval_secs: u64,
    /// Maximum events per API batch.
    #[serde(default = "ReportingSettings::default_batch_size")]
    pub batch_size: usize,
}

impl ReportingSettings {
    fn default_flush_interval() -> u64 {
        60
    }

    fn default_batch_size() -> usize {
        100
    }
}

impl Default for ReportingSettings {
    fn default() -> Self {
        Self {
            flush_interval_secs: Self::default_flush_interval(),
            batch_size: Self::default_batch_size(),
        }
    }
}

/// Top-level agent configuration, loadable from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Device ID assigned after registration.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Enrollment token for initial registration.
    #[serde(default)]
    pub enrollment_token: Option<String>,
    /// API base URL.
    #[serde(default = "AgentConfig::default_api_url")]
    pub api_url: String,
    /// DNS configuration.
    #[serde(default)]
    pub dns: DnsConfig,
    /// Per-plugin configuration, keyed by plugin ID.
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
    /// Reporting settings.
    #[serde(default)]
    pub reporting: ReportingSettings,
    /// Directory for data files (blocklist cache, events DB, etc.).
    #[serde(default = "AgentConfig::default_data_dir")]
    pub data_dir: PathBuf,
    /// Log level.
    #[serde(default = "AgentConfig::default_log_level")]
    pub log_level: String,
}

impl AgentConfig {
    fn default_api_url() -> String {
        "https://api.betblocker.org".into()
    }

    fn default_data_dir() -> PathBuf {
        PathBuf::from("/var/lib/betblocker")
    }

    fn default_log_level() -> String {
        "info".into()
    }

    /// Load configuration from a TOML file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadFailed(path.to_path_buf(), e))?;
        let config: Self =
            toml::from_str(&content).map_err(|e| ConfigError::ParseFailed(e.to_string()))?;
        Ok(config)
    }

    /// Merge enrollment configuration from the server.
    /// Overrides reporting settings and plugin enabled/disabled flags
    /// from the enrollment's server-side configuration.
    pub fn merge_enrollment_config(&mut self, enrollment: &Enrollment) {
        let protection = &enrollment.protection_config;

        // Enable/disable DNS plugins based on protection config
        if let Some(dns_plugin) = self.plugins.get_mut("dns.resolver") {
            dns_plugin.enabled = protection.dns_blocking;
        }
        if let Some(hosts_plugin) = self.plugins.get_mut("dns.hosts") {
            hosts_plugin.enabled = protection.dns_blocking;
        }

        // Enable/disable app blocking plugins (Phase 2)
        if let Some(app_plugin) = self.plugins.get_mut("app.process") {
            app_plugin.enabled = protection.app_blocking;
        }

        // Enable/disable browser blocking plugins (Phase 3)
        if let Some(browser_plugin) = self.plugins.get_mut("browser.extension") {
            browser_plugin.enabled = protection.browser_blocking;
        }
    }

    /// Detect which fields differ between two configs.
    /// Returns a list of field names that changed.
    pub fn has_changed(&self, other: &AgentConfig) -> Vec<String> {
        let mut changes = Vec::new();

        if self.api_url != other.api_url {
            changes.push("api_url".into());
        }
        if self.dns.upstream_servers != other.dns.upstream_servers {
            changes.push("dns.upstream_servers".into());
        }
        if self.dns.listen_addr != other.dns.listen_addr {
            changes.push("dns.listen_addr".into());
        }
        if self.dns.listen_port != other.dns.listen_port {
            changes.push("dns.listen_port".into());
        }
        if self.log_level != other.log_level {
            changes.push("log_level".into());
        }
        if self.data_dir != other.data_dir {
            changes.push("data_dir".into());
        }
        if self.reporting.flush_interval_secs != other.reporting.flush_interval_secs {
            changes.push("reporting.flush_interval_secs".into());
        }
        if self.reporting.batch_size != other.reporting.batch_size {
            changes.push("reporting.batch_size".into());
        }

        // Check plugin config changes
        for (id, config) in &self.plugins {
            match other.plugins.get(id) {
                Some(other_config) => {
                    if config.enabled != other_config.enabled {
                        changes.push(format!("plugins.{id}.enabled"));
                    }
                    if config.priority != other_config.priority {
                        changes.push(format!("plugins.{id}.priority"));
                    }
                }
                None => {
                    changes.push(format!("plugins.{id} (removed)"));
                }
            }
        }

        for id in other.plugins.keys() {
            if !self.plugins.contains_key(id) {
                changes.push(format!("plugins.{id} (added)"));
            }
        }

        changes
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            enrollment_token: None,
            api_url: Self::default_api_url(),
            dns: DnsConfig::default(),
            plugins: HashMap::new(),
            reporting: ReportingSettings::default(),
            data_dir: Self::default_data_dir(),
            log_level: Self::default_log_level(),
        }
    }
}

/// Errors from configuration loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {0}: {1}")]
    ReadFailed(PathBuf, std::io::Error),
    #[error("Failed to parse config: {0}")]
    ParseFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_toml() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("agent.toml");

        let toml_content = r#"
api_url = "https://api.example.com"
log_level = "debug"
data_dir = "/tmp/betblocker"

[dns]
upstream_servers = ["9.9.9.9:53"]
listen_addr = "0.0.0.0"
listen_port = 5353

[reporting]
flush_interval_secs = 30
batch_size = 50

[plugins.dns_resolver]
enabled = true
priority = 10
settings = {}
"#;

        std::fs::write(&config_path, toml_content).expect("write");

        let config = AgentConfig::load(&config_path).expect("load");
        assert_eq!(config.api_url, "https://api.example.com");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.dns.upstream_servers, vec!["9.9.9.9:53"]);
        assert_eq!(config.dns.listen_addr, "0.0.0.0");
        assert_eq!(config.dns.listen_port, 5353);
        assert_eq!(config.reporting.flush_interval_secs, 30);
        assert_eq!(config.reporting.batch_size, 50);
    }

    #[test]
    fn test_defaults_for_missing_fields() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("agent.toml");

        // Minimal TOML with no optional fields
        std::fs::write(&config_path, "").expect("write");

        let config = AgentConfig::load(&config_path).expect("load");
        assert_eq!(config.api_url, "https://api.betblocker.org");
        assert_eq!(config.dns.listen_port, 53);
        assert_eq!(config.log_level, "info");
        assert!(config.device_id.is_none());
    }

    #[test]
    fn test_change_detection() {
        let config1 = AgentConfig::default();
        let mut config2 = config1.clone();

        // No changes
        assert!(config1.has_changed(&config2).is_empty());

        // Change API URL
        config2.api_url = "https://other.api.com".into();
        let changes = config1.has_changed(&config2);
        assert!(changes.contains(&"api_url".to_string()));

        // Change DNS port
        config2.dns.listen_port = 5353;
        let changes = config1.has_changed(&config2);
        assert!(changes.contains(&"dns.listen_port".to_string()));
    }

    #[test]
    fn test_merge_enrollment_config() {
        use bb_common::enums::*;
        use bb_common::models::*;

        let mut config = AgentConfig::default();
        config.plugins.insert(
            "dns.resolver".into(),
            PluginConfig {
                enabled: true,
                ..Default::default()
            },
        );
        config.plugins.insert(
            "dns.hosts".into(),
            PluginConfig {
                enabled: true,
                ..Default::default()
            },
        );

        let enrollment = Enrollment {
            id: 1,
            public_id: uuid::Uuid::nil(),
            device_id: 1,
            account_id: 1,
            enrolled_by: 1,
            tier: EnrollmentTier::SelfEnrolled,
            status: EnrollmentStatus::Active,
            protection_config: ProtectionConfig {
                dns_blocking: false, // Disable DNS
                ..Default::default()
            },
            reporting_config: ReportingConfig::default(),
            unenrollment_policy: UnenrollmentPolicy {
                policy_type: UnenrollmentPolicyType::TimeDelayed,
                cooldown_hours: Some(48),
                requires_approval_from: None,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            expires_at: None,
        };

        config.merge_enrollment_config(&enrollment);

        assert!(
            !config.plugins["dns.resolver"].enabled,
            "DNS resolver should be disabled"
        );
        assert!(
            !config.plugins["dns.hosts"].enabled,
            "DNS hosts should be disabled"
        );
    }
}
