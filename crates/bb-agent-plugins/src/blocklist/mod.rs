pub mod app_signatures;
pub mod cache;

use std::collections::HashSet;

use crate::blocklist::app_signatures::{AppSignatureStore, AppSignatureSummary};
use crate::types::{AppIdentifier, AppMatch};

/// The in-memory blocklist used by all plugins for domain checks.
/// Exact domains go in a `HashSet` for O(1) lookup.
/// Wildcard patterns (e.g., `*.bet365.com`) are stored separately.
#[derive(Debug, Clone)]
pub struct Blocklist {
    /// Exact domain matches (lowercase, no trailing dot).
    exact: HashSet<String>,
    /// Wildcard patterns stored as suffix strings.
    /// e.g., `*.bet365.com` is stored as `.bet365.com`
    /// so we can check if a domain ends with the suffix.
    wildcard_suffixes: Vec<String>,
    /// Blocklist version from the API (for delta sync).
    pub version: i64,
    /// App signature store for application-level blocking.
    app_sigs: AppSignatureStore,
}

impl Blocklist {
    pub fn new(version: i64) -> Self {
        Self {
            exact: HashSet::new(),
            wildcard_suffixes: Vec::new(),
            version,
            app_sigs: AppSignatureStore::new(),
        }
    }

    /// Load from a newline-delimited file of domains.
    /// Lines starting with `*.` are treated as wildcard patterns.
    /// Empty lines and lines starting with `#` are skipped.
    pub fn from_file(path: &std::path::Path, version: i64) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_str_content(&content, version))
    }

    /// Parse from a newline-delimited string of domains.
    pub fn from_str_content(content: &str, version: i64) -> Self {
        let mut blocklist = Self::new(version);
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            blocklist.add_entry(line);
        }
        blocklist
    }

    /// Add a single domain or wildcard pattern.
    pub fn add_entry(&mut self, entry: &str) {
        let entry = entry.to_lowercase();
        let entry = entry.trim_end_matches('.');

        if let Some(suffix) = entry.strip_prefix("*.") {
            // Wildcard: store as ".suffix" for endsWith matching
            self.wildcard_suffixes.push(format!(".{suffix}"));
        } else {
            self.exact.insert(entry.to_string());
        }
    }

    /// Remove a domain or wildcard pattern.
    pub fn remove_entry(&mut self, entry: &str) {
        let entry = entry.to_lowercase();
        let entry = entry.trim_end_matches('.');

        if let Some(suffix) = entry.strip_prefix("*.") {
            let needle = format!(".{suffix}");
            self.wildcard_suffixes.retain(|s| s != &needle);
        } else {
            self.exact.remove(entry);
        }
    }

    /// Check if a domain is blocked.
    /// Checks exact match first, then walks parent domains,
    /// then checks wildcard suffixes.
    pub fn is_blocked(&self, domain: &str) -> bool {
        let domain = domain.to_lowercase();
        let domain = domain.trim_end_matches('.');

        // 1. Exact match on the full domain
        if self.exact.contains(domain) {
            return true;
        }

        // 2. Walk parent domains: sub.bet365.com -> bet365.com -> com
        //    This ensures sub.bet365.com is blocked when bet365.com is in the list.
        let mut remaining = domain;
        while let Some(pos) = remaining.find('.') {
            remaining = &remaining[pos + 1..];
            if self.exact.contains(remaining) {
                return true;
            }
        }

        // 3. Wildcard suffix matching
        for suffix in &self.wildcard_suffixes {
            if domain.ends_with(suffix.as_str()) {
                return true;
            }
        }

        false
    }

    /// Number of entries (exact + wildcard).
    pub fn len(&self) -> usize {
        self.exact.len() + self.wildcard_suffixes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.wildcard_suffixes.is_empty()
    }

    /// Iterate over all exact domain entries.
    pub fn exact_domains(&self) -> impl Iterator<Item = &str> {
        self.exact.iter().map(String::as_str)
    }

    /// Iterate over all wildcard suffix entries (in `.suffix` form).
    pub fn wildcard_suffixes(&self) -> &[String] {
        &self.wildcard_suffixes
    }

    /// Replace the current app signatures with a new set.
    pub fn update_app_signatures(&mut self, sigs: Vec<AppSignatureSummary>) {
        self.app_sigs = AppSignatureStore::from_summaries(sigs);
    }

    /// Check if an application matches any loaded app signature.
    pub fn check_app(&self, app_id: &AppIdentifier) -> Option<AppMatch> {
        self.app_sigs.check_app(app_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_domain_match() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        assert!(bl.is_blocked("bet365.com"));
    }

    #[test]
    fn test_subdomain_match_via_parent_walk() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        assert!(bl.is_blocked("www.bet365.com"));
        assert!(bl.is_blocked("sub.deep.bet365.com"));
    }

    #[test]
    fn test_wildcard_match() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("*.gambling-site.com");
        assert!(bl.is_blocked("app.gambling-site.com"));
        // Wildcard requires a subdomain prefix
        assert!(!bl.is_blocked("gambling-site.com"));
    }

    #[test]
    fn test_non_gambling_domains_pass_through() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        assert!(!bl.is_blocked("google.com"));
        assert!(!bl.is_blocked("github.com"));
    }

    #[test]
    fn test_case_insensitivity() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("Bet365.COM");
        assert!(bl.is_blocked("bet365.com"));
        assert!(bl.is_blocked("BET365.com"));
    }

    #[test]
    fn test_trailing_dot_normalization() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com.");
        assert!(bl.is_blocked("bet365.com"));
        assert!(bl.is_blocked("bet365.com."));
    }

    #[test]
    fn test_from_str_content_loading() {
        let content = "# Comment line\n\nbet365.com\n*.pokerstars.com\ngoogle.com\n";
        let bl = Blocklist::from_str_content(content, 42);
        assert_eq!(bl.version, 42);
        assert_eq!(bl.len(), 3);
        assert!(bl.is_blocked("bet365.com"));
        assert!(bl.is_blocked("www.pokerstars.com"));
        assert!(bl.is_blocked("google.com"));
        assert!(!bl.is_blocked("pokerstars.com")); // wildcard needs subdomain
    }

    #[test]
    fn test_from_file_loading() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("blocklist.txt");
        std::fs::write(
            &file_path,
            "# BetBlocker blocklist\n\nbet365.com\n*.example-gambling.com\n\n",
        )
        .expect("Failed to write file");

        let bl = Blocklist::from_file(&file_path, 10).expect("Failed to load");
        assert_eq!(bl.len(), 2);
        assert!(bl.is_blocked("bet365.com"));
        assert!(bl.is_blocked("sub.example-gambling.com"));
    }

    #[test]
    fn test_remove_entry() {
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        bl.add_entry("*.pokerstars.com");
        assert!(bl.is_blocked("bet365.com"));
        assert!(bl.is_blocked("www.pokerstars.com"));

        bl.remove_entry("bet365.com");
        assert!(!bl.is_blocked("bet365.com"));

        bl.remove_entry("*.pokerstars.com");
        assert!(!bl.is_blocked("www.pokerstars.com"));
    }

    #[test]
    fn test_empty_blocklist() {
        let bl = Blocklist::new(0);
        assert!(bl.is_empty());
        assert_eq!(bl.len(), 0);
        assert!(!bl.is_blocked("anything.com"));
    }

    #[test]
    fn test_blocklist_blocks_matching_app() {
        use crate::blocklist::app_signatures::AppSignatureSummary;
        use crate::types::{AppIdentifier, AppMatchType};
        use bb_common::enums::Platform;

        let mut bl = Blocklist::new(1);
        bl.update_app_signatures(vec![AppSignatureSummary {
            public_id: uuid::Uuid::nil(),
            name: "Bet365".to_string(),
            package_names: vec!["com.bet365.app".to_string()],
            executable_names: vec!["bet365.exe".to_string()],
            cert_hashes: vec![],
            display_name_patterns: vec![],
            platforms: vec!["windows".to_string()],
            category: "sports_betting".to_string(),
            confidence: 0.85,
        }]);

        let mut app = AppIdentifier::empty(Platform::Windows);
        app.package_name = Some("com.bet365.app".to_string());
        let result = bl.check_app(&app);
        assert!(result.is_some());
        assert_eq!(result.unwrap().match_type, AppMatchType::ExactPackage);
    }

    #[test]
    fn test_update_replaces_old_signatures() {
        use crate::blocklist::app_signatures::AppSignatureSummary;
        use crate::types::AppIdentifier;
        use bb_common::enums::Platform;

        let mut bl = Blocklist::new(1);

        // Load first set
        bl.update_app_signatures(vec![AppSignatureSummary {
            public_id: uuid::Uuid::nil(),
            name: "OldApp".to_string(),
            package_names: vec!["com.old.app".to_string()],
            executable_names: vec![],
            cert_hashes: vec![],
            display_name_patterns: vec![],
            platforms: vec!["windows".to_string()],
            category: "casino".to_string(),
            confidence: 0.85,
        }]);

        let mut app_old = AppIdentifier::empty(Platform::Windows);
        app_old.package_name = Some("com.old.app".to_string());
        assert!(bl.check_app(&app_old).is_some());

        // Replace with new set
        bl.update_app_signatures(vec![AppSignatureSummary {
            public_id: uuid::Uuid::nil(),
            name: "NewApp".to_string(),
            package_names: vec!["com.new.app".to_string()],
            executable_names: vec![],
            cert_hashes: vec![],
            display_name_patterns: vec![],
            platforms: vec!["windows".to_string()],
            category: "casino".to_string(),
            confidence: 0.85,
        }]);

        // Old app should no longer match
        assert!(bl.check_app(&app_old).is_none());

        // New app should match
        let mut app_new = AppIdentifier::empty(Platform::Windows);
        app_new.package_name = Some("com.new.app".to_string());
        assert!(bl.check_app(&app_new).is_some());
    }
}
