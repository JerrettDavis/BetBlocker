pub mod platform;

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::blocklist::Blocklist;
use crate::traits::{BlockingPlugin, DnsBlockingPlugin};
use crate::types::{BlockDecision, BlockingLayer, PluginConfig, PluginError, PluginHealth};

const BEGIN_MARKER: &str = "# BEGIN BETBLOCKER";
const END_MARKER: &str = "# END BETBLOCKER";
const MAX_HOSTS_ENTRIES: usize = 5000;

/// Plugin that writes blocked domains to the system HOSTS file as a fallback
/// blocking mechanism. Entries are written as `0.0.0.0 <domain>` between
/// marker comments for easy identification and removal.
pub struct HostsFilePlugin {
    hosts_path: PathBuf,
    blocklist: Option<Arc<Blocklist>>,
    active: bool,
    /// SHA-256 hash of the BetBlocker section for tamper detection.
    entries_hash: Option<String>,
}

impl HostsFilePlugin {
    pub fn new() -> Self {
        Self {
            hosts_path: platform::hosts_file_path(),
            blocklist: None,
            active: false,
            entries_hash: None,
        }
    }

    /// Write BetBlocker entries into the HOSTS file.
    fn write_entries(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        let content = std::fs::read_to_string(&self.hosts_path).unwrap_or_default();

        // Remove existing BetBlocker section
        let cleaned = remove_betblocker_section(&content);

        // Build the new BetBlocker section
        let section = build_betblocker_section(blocklist);

        // Compute hash for tamper detection
        self.entries_hash = Some(sha256_hex(&section));

        // Combine cleaned content with new section
        let mut new_content = cleaned.trim_end().to_string();
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(&section);
        new_content.push('\n');

        // Write atomically via temp file + rename
        let tmp_path = self.hosts_path.with_extension("betblocker.tmp");
        let mut file = std::fs::File::create(&tmp_path).map_err(PluginError::Io)?;
        file.write_all(new_content.as_bytes())
            .map_err(PluginError::Io)?;
        file.flush().map_err(PluginError::Io)?;
        drop(file);

        std::fs::rename(&tmp_path, &self.hosts_path).or_else(|_| {
            // On Windows, rename may fail if target is locked.
            // Fall back to direct write.
            std::fs::write(&self.hosts_path, new_content.as_bytes()).map_err(PluginError::Io)
        })?;

        info!(
            entries = blocklist.len().min(MAX_HOSTS_ENTRIES),
            "Wrote BetBlocker entries to HOSTS file"
        );
        Ok(())
    }

    /// Remove BetBlocker entries from the HOSTS file.
    fn remove_entries(&mut self) -> Result<(), PluginError> {
        let content = std::fs::read_to_string(&self.hosts_path).map_err(PluginError::Io)?;
        let cleaned = remove_betblocker_section(&content);

        std::fs::write(&self.hosts_path, cleaned.as_bytes()).map_err(PluginError::Io)?;
        self.entries_hash = None;

        info!("Removed BetBlocker entries from HOSTS file");
        Ok(())
    }

    /// Check if the HOSTS file has been tampered with by comparing the current
    /// BetBlocker section hash against the stored hash.
    fn check_tamper(&self) -> Result<bool, PluginError> {
        let Some(expected_hash) = &self.entries_hash else {
            return Ok(false); // No entries written, nothing to check
        };

        let content = std::fs::read_to_string(&self.hosts_path).map_err(PluginError::Io)?;
        let section = extract_betblocker_section(&content);
        let current_hash = sha256_hex(&section);

        Ok(&current_hash != expected_hash)
    }

    /// Check for tamper and restore if needed. Returns true if restoration was performed.
    pub fn check_and_restore(&mut self) -> Result<bool, PluginError> {
        let tampered = self.check_tamper()?;
        if tampered {
            warn!("HOSTS file tamper detected, restoring BetBlocker entries");
            if let Some(bl) = &self.blocklist {
                let bl = Arc::clone(bl);
                self.write_entries(&bl)?;
            }
            return Ok(true);
        }
        Ok(false)
    }
}

impl Default for HostsFilePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HostsFilePlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostsFilePlugin")
            .field("hosts_path", &self.hosts_path)
            .field("active", &self.active)
            .finish()
    }
}

impl BlockingPlugin for HostsFilePlugin {
    fn id(&self) -> &str {
        "dns.hosts"
    }

    fn name(&self) -> &str {
        "HOSTS File"
    }

    fn layer(&self) -> BlockingLayer {
        BlockingLayer::Dns
    }

    fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError> {
        // Allow overriding the hosts path for testing
        if let Some(path) = config.settings.get("hosts_file_path")
            && let Some(path_str) = path.as_str()
        {
            self.hosts_path = PathBuf::from(path_str);
        }

        info!(hosts_path = %self.hosts_path.display(), "HOSTS file plugin initialized");
        Ok(())
    }

    fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        self.blocklist = Some(Arc::new(blocklist.clone()));
        self.write_entries(blocklist)?;
        self.active = true;
        info!("HOSTS file plugin activated");
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), PluginError> {
        self.remove_entries()?;
        self.blocklist = None;
        self.active = false;
        info!("HOSTS file plugin deactivated");
        Ok(())
    }

    fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        self.blocklist = Some(Arc::new(blocklist.clone()));
        self.write_entries(blocklist)?;
        info!(version = blocklist.version, "HOSTS file blocklist updated");
        Ok(())
    }

    fn health_check(&self) -> Result<PluginHealth, PluginError> {
        if !self.active {
            return Ok(PluginHealth::degraded("HOSTS file plugin is not active"));
        }

        match self.check_tamper() {
            Ok(true) => Ok(PluginHealth::degraded("HOSTS file tampered")),
            Ok(false) => Ok(PluginHealth::ok()),
            Err(e) => Err(e),
        }
    }
}

impl DnsBlockingPlugin for HostsFilePlugin {
    fn check_domain(&self, domain: &str) -> BlockDecision {
        // The HOSTS file plugin's blocking happens at the OS level.
        // This check delegates to the in-memory blocklist for consistency.
        match &self.blocklist {
            Some(bl) if bl.is_blocked(domain) => BlockDecision::Block {
                reason: format!("Domain '{domain}' blocked via HOSTS file"),
            },
            Some(_) => BlockDecision::Allow,
            None => BlockDecision::Abstain,
        }
    }

    fn handle_dns_query(&self, _query: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

/// Build the BetBlocker section content for the HOSTS file.
fn build_betblocker_section(blocklist: &Blocklist) -> String {
    use std::fmt::Write;

    let mut section = String::new();
    section.push_str(BEGIN_MARKER);
    section.push('\n');
    let _ = writeln!(
        section,
        "# Generated by BetBlocker at {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    let _ = writeln!(section, "# Blocklist version: {}", blocklist.version);

    let mut domains: Vec<&str> = blocklist.exact_domains().collect();
    domains.sort_unstable();

    // Limit entries for HOSTS file performance
    if domains.len() > MAX_HOSTS_ENTRIES {
        warn!(
            total = domains.len(),
            max = MAX_HOSTS_ENTRIES,
            "Blocklist exceeds HOSTS file limit, truncating"
        );
    }

    for domain in domains.iter().take(MAX_HOSTS_ENTRIES) {
        let _ = writeln!(section, "0.0.0.0 {domain}");
    }

    section.push_str(END_MARKER);
    section
}

/// Remove the BetBlocker section from HOSTS file content.
fn remove_betblocker_section(content: &str) -> String {
    let mut result = String::new();
    let mut in_section = false;

    for line in content.lines() {
        if line.trim() == BEGIN_MARKER {
            in_section = true;
            continue;
        }
        if line.trim() == END_MARKER {
            in_section = false;
            continue;
        }
        if !in_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Extract the BetBlocker section from HOSTS file content.
fn extract_betblocker_section(content: &str) -> String {
    let mut section = String::new();
    let mut in_section = false;

    for line in content.lines() {
        if line.trim() == BEGIN_MARKER {
            in_section = true;
            section.push_str(line);
            section.push('\n');
            continue;
        }
        if line.trim() == END_MARKER {
            section.push_str(line);
            in_section = false;
            continue;
        }
        if in_section {
            section.push_str(line);
            section.push('\n');
        }
    }

    section
}

/// Compute SHA-256 hex digest of a string.
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_blocklist(domains: &[&str]) -> Blocklist {
        let mut bl = Blocklist::new(1);
        for d in domains {
            bl.add_entry(d);
        }
        bl
    }

    fn make_plugin_with_tmp(path: &std::path::Path) -> (HostsFilePlugin, PluginConfig) {
        let mut plugin = HostsFilePlugin::new();
        let mut config = PluginConfig::default();
        config.settings.insert(
            "hosts_file_path".into(),
            serde_json::json!(path.to_str().unwrap()),
        );
        plugin.init(&config).expect("init");
        (plugin, config)
    }

    #[test]
    fn test_write_entries_preserves_existing_content() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hosts_path = dir.path().join("hosts");
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n::1 localhost\n").expect("write");

        let (mut plugin, _config) = make_plugin_with_tmp(&hosts_path);
        let bl = make_blocklist(&["bet365.com", "pokerstars.com"]);
        plugin.activate(&bl).expect("activate");

        let content = std::fs::read_to_string(&hosts_path).expect("read");
        assert!(content.contains("127.0.0.1 localhost"));
        assert!(content.contains("::1 localhost"));
        assert!(content.contains(BEGIN_MARKER));
        assert!(content.contains(END_MARKER));
        assert!(content.contains("0.0.0.0 bet365.com"));
        assert!(content.contains("0.0.0.0 pokerstars.com"));
    }

    #[test]
    fn test_remove_entries_cleans_up() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hosts_path = dir.path().join("hosts");
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("write");

        let (mut plugin, _config) = make_plugin_with_tmp(&hosts_path);
        let bl = make_blocklist(&["bet365.com"]);
        plugin.activate(&bl).expect("activate");

        // Verify entries are present
        let content = std::fs::read_to_string(&hosts_path).expect("read");
        assert!(content.contains("0.0.0.0 bet365.com"));

        // Deactivate removes entries
        plugin.deactivate().expect("deactivate");
        let content = std::fs::read_to_string(&hosts_path).expect("read");
        assert!(content.contains("127.0.0.1 localhost"));
        assert!(!content.contains(BEGIN_MARKER));
        assert!(!content.contains("0.0.0.0 bet365.com"));
    }

    #[test]
    fn test_idempotent_writes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hosts_path = dir.path().join("hosts");
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("write");

        let (mut plugin, _config) = make_plugin_with_tmp(&hosts_path);
        let bl = make_blocklist(&["bet365.com"]);
        plugin.activate(&bl).expect("activate");

        // Write again with same blocklist
        plugin.update_blocklist(&bl).expect("update");

        let content = std::fs::read_to_string(&hosts_path).expect("read");
        // Should only have one BEGIN/END pair
        assert_eq!(
            content.matches(BEGIN_MARKER).count(),
            1,
            "Should have exactly one BetBlocker section"
        );
        assert_eq!(content.matches(END_MARKER).count(), 1);
    }

    #[test]
    fn test_tamper_detection() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hosts_path = dir.path().join("hosts");
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("write");

        let (mut plugin, _config) = make_plugin_with_tmp(&hosts_path);
        let bl = make_blocklist(&["bet365.com"]);
        plugin.activate(&bl).expect("activate");

        // Verify healthy
        let health = plugin.health_check().expect("health");
        assert!(health.healthy);

        // Tamper with the file
        let content = std::fs::read_to_string(&hosts_path).expect("read");
        let tampered = content.replace("0.0.0.0 bet365.com", "# removed by user");
        std::fs::write(&hosts_path, tampered).expect("write");

        // Health check should detect tamper
        let health = plugin.health_check().expect("health");
        assert!(!health.healthy);
        assert!(health.message.contains("tampered"));
    }

    #[test]
    fn test_auto_restore_after_tamper() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hosts_path = dir.path().join("hosts");
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("write");

        let (mut plugin, _config) = make_plugin_with_tmp(&hosts_path);
        let bl = make_blocklist(&["bet365.com"]);
        plugin.activate(&bl).expect("activate");

        // Tamper
        let content = std::fs::read_to_string(&hosts_path).expect("read");
        let tampered = content.replace("0.0.0.0 bet365.com", "# hacked");
        std::fs::write(&hosts_path, tampered).expect("write");

        // Auto-restore
        let restored = plugin.check_and_restore().expect("restore");
        assert!(restored);

        // Verify restored
        let content = std::fs::read_to_string(&hosts_path).expect("read");
        assert!(content.contains("0.0.0.0 bet365.com"));

        // Health should be OK now
        let health = plugin.health_check().expect("health");
        assert!(health.healthy);
    }

    #[test]
    fn test_empty_blocklist() {
        let dir = tempfile::tempdir().expect("temp dir");
        let hosts_path = dir.path().join("hosts");
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("write");

        let (mut plugin, _config) = make_plugin_with_tmp(&hosts_path);
        let bl = Blocklist::new(1);
        plugin.activate(&bl).expect("activate");

        let content = std::fs::read_to_string(&hosts_path).expect("read");
        assert!(content.contains(BEGIN_MARKER));
        assert!(content.contains(END_MARKER));
        // No domain entries
        assert!(!content.contains("0.0.0.0 "));

        // Deactivate
        plugin.deactivate().expect("deactivate");
        let content = std::fs::read_to_string(&hosts_path).expect("read");
        assert!(!content.contains(BEGIN_MARKER));
    }
}
