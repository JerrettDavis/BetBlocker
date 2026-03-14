use tracing::{error, info, warn};

use crate::blocklist::Blocklist;
use crate::types::{BlockDecision, BlockingLayer, PluginConfig, PluginError, PluginHealth};

#[cfg(feature = "app-process")]
use crate::types::{AppIdentifier, AppMatch};

#[cfg(feature = "dns-resolver")]
use crate::dns_resolver::DnsResolverPlugin;

#[cfg(feature = "dns-hosts")]
use crate::hosts_file::HostsFilePlugin;

#[cfg(feature = "app-process")]
use crate::app_process::AppProcessPlugin;

use crate::traits::BlockingPlugin;

#[cfg(feature = "app-process")]
use crate::traits::AppBlockingPlugin;

use crate::traits::DnsBlockingPlugin;

/// Enum dispatch over all compiled-in plugins.
/// Each variant is conditionally compiled via feature flags.
pub enum PluginInstance {
    #[cfg(feature = "dns-resolver")]
    DnsResolver(DnsResolverPlugin),

    #[cfg(feature = "dns-hosts")]
    DnsHosts(HostsFilePlugin),

    #[cfg(feature = "app-process")]
    AppProcess(AppProcessPlugin),
}

/// Macro to dispatch `BlockingPlugin` methods across all `PluginInstance` variants.
macro_rules! dispatch_blocking {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(p) => p.$method($($arg),*),
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(p) => p.$method($($arg),*),
            #[cfg(feature = "app-process")]
            PluginInstance::AppProcess(p) => p.$method($($arg),*),
        }
    };
}

/// Mutable dispatch variant.
macro_rules! dispatch_blocking_mut {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(p) => p.$method($($arg),*),
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(p) => p.$method($($arg),*),
            #[cfg(feature = "app-process")]
            PluginInstance::AppProcess(p) => p.$method($($arg),*),
        }
    };
}

impl PluginInstance {
    pub fn id(&self) -> &str {
        dispatch_blocking!(self, id)
    }

    pub fn name(&self) -> &str {
        dispatch_blocking!(self, name)
    }

    pub fn layer(&self) -> BlockingLayer {
        dispatch_blocking!(self, layer)
    }

    pub fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError> {
        dispatch_blocking_mut!(self, init, config)
    }

    pub fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        dispatch_blocking_mut!(self, activate, blocklist)
    }

    pub fn deactivate(&mut self) -> Result<(), PluginError> {
        dispatch_blocking_mut!(self, deactivate)
    }

    pub fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        dispatch_blocking_mut!(self, update_blocklist, blocklist)
    }

    pub fn health_check(&self) -> Result<PluginHealth, PluginError> {
        dispatch_blocking!(self, health_check)
    }

    /// Returns true if this plugin is a DNS-layer plugin.
    pub fn is_dns_plugin(&self) -> bool {
        self.layer() == BlockingLayer::Dns
    }

    /// Check domain against this plugin, if it supports DNS blocking.
    /// Returns `Abstain` for non-DNS plugins.
    pub fn check_domain(&self, domain: &str) -> BlockDecision {
        match self {
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(p) => p.check_domain(domain),
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(p) => p.check_domain(domain),
            #[cfg(feature = "app-process")]
            PluginInstance::AppProcess(_p) => BlockDecision::Abstain,
        }
    }

    /// Returns true if this plugin is an application-layer plugin.
    #[cfg(feature = "app-process")]
    pub fn is_app_plugin(&self) -> bool {
        self.layer() == BlockingLayer::App
    }

    /// Check an application identifier against this plugin, if it supports app blocking.
    /// Returns `Abstain` for non-app plugins.
    #[cfg(feature = "app-process")]
    pub fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision {
        match self {
            PluginInstance::AppProcess(p) => p.check_app(app_id),
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(_p) => BlockDecision::Abstain,
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(_p) => BlockDecision::Abstain,
        }
    }

    /// Scan installed applications and return blocked matches (app plugins only).
    #[cfg(feature = "app-process")]
    pub fn scan_apps(&self) -> Vec<AppMatch> {
        match self {
            PluginInstance::AppProcess(p) => p.scan_installed(),
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(_p) => Vec::new(),
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(_p) => Vec::new(),
        }
    }
}

/// Manages the lifecycle of all compiled-in plugins.
pub struct PluginRegistry {
    plugins: Vec<PluginInstance>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin instance.
    pub fn register(&mut self, plugin: PluginInstance) {
        info!(plugin_id = plugin.id(), "Registered plugin");
        self.plugins.push(plugin);
    }

    /// Build the default registry with all compiled-in plugins.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        #[cfg(feature = "dns-resolver")]
        registry.register(PluginInstance::DnsResolver(DnsResolverPlugin::new()));

        #[cfg(feature = "dns-hosts")]
        registry.register(PluginInstance::DnsHosts(HostsFilePlugin::new()));

        #[cfg(feature = "app-process")]
        registry.register(PluginInstance::AppProcess(AppProcessPlugin::new()));

        registry
    }

    /// Initialize all plugins. Returns errors for plugins that failed init.
    /// Failed plugins are removed from the registry.
    pub fn init_all(
        &mut self,
        config: &PluginConfig,
        blocklist: &Blocklist,
    ) -> Vec<PluginError> {
        let mut errors = Vec::new();
        let mut i = 0;
        while i < self.plugins.len() {
            let plugin = &mut self.plugins[i];
            let id = plugin.id().to_string();

            match plugin.init(config) {
                Ok(()) => {
                    info!(plugin_id = %id, "Plugin initialized");
                    match plugin.activate(blocklist) {
                        Ok(()) => {
                            info!(plugin_id = %id, "Plugin activated");
                            i += 1;
                        }
                        Err(e) => {
                            error!(plugin_id = %id, error = %e, "Plugin activation failed, removing");
                            errors.push(e);
                            self.plugins.remove(i);
                        }
                    }
                }
                Err(e) => {
                    error!(plugin_id = %id, error = %e, "Plugin init failed, removing");
                    errors.push(e);
                    self.plugins.remove(i);
                }
            }
        }
        errors
    }

    /// Query all DNS plugins for a domain.
    /// Short-circuits on first Block decision for performance.
    pub fn check_domain(&self, domain: &str) -> BlockDecision {
        for plugin in &self.plugins {
            if !plugin.is_dns_plugin() {
                continue;
            }
            let decision = plugin.check_domain(domain);
            if decision.is_blocked() {
                return decision;
            }
        }
        BlockDecision::Allow
    }

    /// Query all app-layer plugins for an application identifier.
    /// Short-circuits on first Block decision.
    #[cfg(feature = "app-process")]
    pub fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision {
        for plugin in &self.plugins {
            if !plugin.is_app_plugin() {
                continue;
            }
            let decision = plugin.check_app(app_id);
            if decision.is_blocked() {
                return decision;
            }
        }
        BlockDecision::Allow
    }

    /// Scan all app-layer plugins for installed blocked applications.
    /// Aggregates results from all plugins.
    #[cfg(feature = "app-process")]
    pub fn scan_all_apps(&self) -> Vec<AppMatch> {
        let mut results = Vec::new();
        for plugin in &self.plugins {
            if plugin.is_app_plugin() {
                results.extend(plugin.scan_apps());
            }
        }
        results
    }

    /// Run health checks on all plugins. Returns (plugin_id, error) for failures.
    pub fn health_check_all(&self) -> Vec<(String, PluginError)> {
        let mut failures = Vec::new();
        for plugin in &self.plugins {
            match plugin.health_check() {
                Ok(health) if !health.healthy => {
                    failures.push((
                        plugin.id().to_string(),
                        PluginError::Unhealthy(health.message),
                    ));
                }
                Err(e) => {
                    failures.push((plugin.id().to_string(), e));
                }
                Ok(_) => {}
            }
        }
        failures
    }

    /// Push updated blocklist to all active plugins.
    pub fn update_blocklist_all(&mut self, blocklist: &Blocklist) -> Vec<PluginError> {
        let mut errors = Vec::new();
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.update_blocklist(blocklist) {
                warn!(plugin_id = plugin.id(), error = %e, "Blocklist update failed");
                errors.push(e);
            }
        }
        errors
    }

    /// Gracefully deactivate all plugins (shutdown path).
    pub fn deactivate_all(&mut self) -> Vec<PluginError> {
        let mut errors = Vec::new();
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.deactivate() {
                warn!(plugin_id = plugin.id(), error = %e, "Plugin deactivation failed");
                errors.push(e);
            }
        }
        errors
    }

    /// Number of active plugins.
    pub fn active_count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_registry_new_is_empty() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.active_count(), 0);
    }

    #[cfg(feature = "dns-resolver")]
    #[test]
    fn test_registry_check_domain_returns_allow_when_empty() {
        let registry = PluginRegistry::new();
        let decision = registry.check_domain("bet365.com");
        assert_eq!(decision, BlockDecision::Allow);
    }

    #[cfg(feature = "dns-resolver")]
    #[test]
    fn test_registry_with_defaults_has_plugins() {
        let registry = PluginRegistry::with_defaults();
        // With both features enabled, we should have 2 plugins
        assert!(registry.active_count() >= 1);
    }

    #[cfg(feature = "dns-resolver")]
    #[test]
    fn test_registry_deactivate_all() {
        let mut registry = PluginRegistry::new();
        let plugin = crate::dns_resolver::DnsResolverPlugin::new();
        // Don't activate, just register
        registry.register(PluginInstance::DnsResolver(plugin));

        let errors = registry.deactivate_all();
        // Deactivating an inactive plugin should not error
        assert!(errors.is_empty());
    }

    #[cfg(feature = "app-process")]
    #[test]
    fn test_registry_app_process_variant_registers() {
        use crate::app_process::AppProcessPlugin;
        let mut registry = PluginRegistry::new();
        registry.register(PluginInstance::AppProcess(AppProcessPlugin::new()));
        assert_eq!(registry.active_count(), 1);
    }

    #[cfg(feature = "app-process")]
    #[test]
    fn test_registry_check_app_returns_allow_when_empty() {
        use crate::types::AppIdentifier;
        use bb_common::enums::Platform;
        let registry = PluginRegistry::new();
        let app = AppIdentifier::empty(Platform::Windows);
        let decision = registry.check_app(&app);
        assert_eq!(decision, BlockDecision::Allow);
    }

    #[cfg(feature = "app-process")]
    #[test]
    fn test_registry_scan_all_apps_returns_empty_with_noop() {
        use crate::app_process::AppProcessPlugin;
        let mut registry = PluginRegistry::new();
        registry.register(PluginInstance::AppProcess(AppProcessPlugin::new()));
        let matches = registry.scan_all_apps();
        assert!(matches.is_empty());
    }

    #[cfg(feature = "app-process")]
    #[test]
    fn test_plugin_instance_is_app_plugin() {
        use crate::app_process::AppProcessPlugin;
        let instance = PluginInstance::AppProcess(AppProcessPlugin::new());
        assert!(instance.is_app_plugin());
        assert!(!instance.is_dns_plugin());
    }

    #[cfg(feature = "app-process")]
    #[test]
    fn test_registry_with_defaults_includes_app_process() {
        let registry = PluginRegistry::with_defaults();
        // with_defaults() adds dns-resolver, dns-hosts, and app-process
        let has_app = registry.plugins.iter().any(|p| p.id() == "app.process");
        assert!(has_app);
    }
}
