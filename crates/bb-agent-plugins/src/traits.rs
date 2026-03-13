use crate::blocklist::Blocklist;
use crate::types::{
    AppIdentifier, AppMatch, BlockDecision, BlockingLayer, ContentRules, ExtensionHealth,
    PluginConfig, PluginError, PluginHealth,
};

/// Base trait for all blocking plugins. Handles lifecycle management.
pub trait BlockingPlugin: Send + Sync + 'static {
    /// Unique identifier, e.g., "dns.resolver", "dns.hosts".
    fn id(&self) -> &str;

    /// Human-readable name for logging and status reporting.
    fn name(&self) -> &str;

    /// Which blocking layer this plugin belongs to.
    fn layer(&self) -> BlockingLayer;

    /// Initialize with configuration. Called once at agent startup.
    /// May fail if OS prerequisites are missing.
    fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError>;

    /// Activate blocking. Called after init, once the blocklist is loaded.
    fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError>;

    /// Deactivate blocking. Called on graceful shutdown or plugin hot-reload.
    fn deactivate(&mut self) -> Result<(), PluginError>;

    /// Receive an updated blocklist after a delta sync completes.
    fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError>;

    /// Health check, called periodically by the watchdog.
    fn health_check(&self) -> Result<PluginHealth, PluginError>;
}

/// DNS/Network layer plugins implement this in addition to `BlockingPlugin`.
pub trait DnsBlockingPlugin: BlockingPlugin {
    /// Check if a domain should be blocked.
    /// Must be extremely fast (sub-microsecond for cache hit).
    fn check_domain(&self, domain: &str) -> BlockDecision;

    /// Handle a raw DNS query packet. For plugins that operate at the
    /// packet level (WFP, `VpnService`). Returns None if this plugin
    /// does not handle raw packets.
    fn handle_dns_query(&self, query: &[u8]) -> Option<Vec<u8>>;
}

/// Application layer plugins (Phase 2).
pub trait AppBlockingPlugin: BlockingPlugin {
    /// Check if an application should be blocked.
    fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision;

    /// Scan installed applications and return matches.
    fn scan_installed(&self) -> Vec<AppMatch>;

    /// Start monitoring for new app installations.
    fn watch_installs(&mut self) -> Result<(), PluginError>;
}

/// Browser/Content layer plugins (Phase 3).
pub trait ContentBlockingPlugin: BlockingPlugin {
    /// Generate content blocking rules for browser extensions.
    fn generate_rules(&self, blocklist: &Blocklist) -> ContentRules;

    /// Check browser extension presence and integrity.
    fn check_extension_health(&self) -> ExtensionHealth;
}
