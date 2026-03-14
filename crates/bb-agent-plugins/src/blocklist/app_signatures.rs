use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{AppIdentifier, AppMatch, AppMatchType};

/// Lightweight version of `AppSignature` used for matching on the agent side.
/// Contains only the fields needed for matching, avoiding heavy DB-related fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSignatureSummary {
    pub public_id: Uuid,
    pub name: String,
    pub package_names: Vec<String>,
    pub executable_names: Vec<String>,
    pub cert_hashes: Vec<String>,
    pub display_name_patterns: Vec<String>,
    pub platforms: Vec<String>,
    pub category: String,
    pub confidence: f64,
}

/// In-memory store for app signatures with pre-built indexes for fast lookup.
#[derive(Debug, Clone)]
pub struct AppSignatureStore {
    /// All loaded signatures.
    signatures: Vec<AppSignatureSummary>,
    /// Index: lowercased package name -> list of signature indices.
    package_index: HashMap<String, Vec<usize>>,
    /// Index: lowercased executable name -> list of signature indices.
    executable_index: HashMap<String, Vec<usize>>,
    /// Index: cert hash (exact, lowercased) -> list of signature indices.
    cert_hash_index: HashMap<String, Vec<usize>>,
}

impl AppSignatureStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            signatures: Vec::new(),
            package_index: HashMap::new(),
            executable_index: HashMap::new(),
            cert_hash_index: HashMap::new(),
        }
    }

    /// Build a store from a list of signature summaries, creating lookup indexes.
    pub fn from_summaries(sigs: Vec<AppSignatureSummary>) -> Self {
        let mut store = Self {
            signatures: Vec::with_capacity(sigs.len()),
            package_index: HashMap::new(),
            executable_index: HashMap::new(),
            cert_hash_index: HashMap::new(),
        };

        for (idx, sig) in sigs.into_iter().enumerate() {
            for pkg in &sig.package_names {
                store
                    .package_index
                    .entry(pkg.to_lowercase())
                    .or_default()
                    .push(idx);
            }
            for exe in &sig.executable_names {
                store
                    .executable_index
                    .entry(exe.to_lowercase())
                    .or_default()
                    .push(idx);
            }
            for hash in &sig.cert_hashes {
                store
                    .cert_hash_index
                    .entry(hash.to_lowercase())
                    .or_default()
                    .push(idx);
            }
            store.signatures.push(sig);
        }

        store
    }

    /// Number of loaded signatures.
    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    /// Check if a package name matches any signature (case-insensitive exact match).
    pub fn check_package_name(&self, name: &str, app_id: &AppIdentifier) -> Option<AppMatch> {
        let key = name.to_lowercase();
        let indices = self.package_index.get(&key)?;
        let sig = &self.signatures[indices[0]];
        Some(AppMatch {
            app_id: app_id.clone(),
            signature_id: sig.public_id,
            signature_name: sig.name.clone(),
            match_type: AppMatchType::ExactPackage,
            confidence: 1.0,
            reason: format!("Package name '{}' matches signature '{}'", name, sig.name),
        })
    }

    /// Check if an executable name matches any signature (case-insensitive exact match).
    pub fn check_executable(&self, exe_name: &str, app_id: &AppIdentifier) -> Option<AppMatch> {
        let key = exe_name.to_lowercase();
        let indices = self.executable_index.get(&key)?;
        let sig = &self.signatures[indices[0]];
        Some(AppMatch {
            app_id: app_id.clone(),
            signature_id: sig.public_id,
            signature_name: sig.name.clone(),
            match_type: AppMatchType::ExactExecutable,
            confidence: 1.0,
            reason: format!(
                "Executable name '{}' matches signature '{}'",
                exe_name, sig.name
            ),
        })
    }

    /// Check if a cert hash matches any signature (exact match, case-insensitive).
    pub fn check_cert_hash(&self, hash: &str, app_id: &AppIdentifier) -> Option<AppMatch> {
        let key = hash.to_lowercase();
        let indices = self.cert_hash_index.get(&key)?;
        let sig = &self.signatures[indices[0]];
        Some(AppMatch {
            app_id: app_id.clone(),
            signature_id: sig.public_id,
            signature_name: sig.name.clone(),
            match_type: AppMatchType::CertHash,
            confidence: 1.0,
            reason: format!("Cert hash matches signature '{}'", sig.name),
        })
    }

    /// Check if a display name fuzzy-matches any signature's display name patterns.
    /// Uses normalized Levenshtein distance; matches when similarity >= signature's confidence
    /// (default threshold 0.85).
    pub fn check_display_name(
        &self,
        display_name: &str,
        app_id: &AppIdentifier,
    ) -> Option<AppMatch> {
        let query = display_name.to_lowercase();
        let mut best_match: Option<(usize, f64)> = None;

        for (idx, sig) in self.signatures.iter().enumerate() {
            let threshold = if sig.confidence > 0.0 {
                sig.confidence
            } else {
                0.85
            };

            for pattern in &sig.display_name_patterns {
                let pattern_lower = pattern.to_lowercase();
                let similarity = strsim::normalized_levenshtein(&query, &pattern_lower);

                if similarity >= threshold {
                    if best_match.map_or(true, |(_, best_sim)| similarity > best_sim) {
                        best_match = Some((idx, similarity));
                    }
                }
            }
        }

        let (idx, similarity) = best_match?;
        let sig = &self.signatures[idx];
        Some(AppMatch {
            app_id: app_id.clone(),
            signature_id: sig.public_id,
            signature_name: sig.name.clone(),
            match_type: AppMatchType::FuzzyDisplayName,
            confidence: similarity,
            reason: format!(
                "Display name '{}' fuzzy-matches signature '{}' (similarity: {:.2})",
                display_name, sig.name, similarity
            ),
        })
    }

    /// Check an `AppIdentifier` against all matching strategies.
    /// Tries in order: cert_hash > package_name > executable > display_name.
    /// Returns the first match found.
    pub fn check_app(&self, app_id: &AppIdentifier) -> Option<AppMatch> {
        // 1. Cert hash (highest trust)
        if let Some(hash) = &app_id.cert_hash {
            if let Some(m) = self.check_cert_hash(hash, app_id) {
                return Some(m);
            }
        }

        // 2. Package name
        if let Some(pkg) = &app_id.package_name {
            if let Some(m) = self.check_package_name(pkg, app_id) {
                return Some(m);
            }
        }

        // 3. Executable name
        if let Some(exe) = &app_id.executable_name {
            if let Some(m) = self.check_executable(exe, app_id) {
                return Some(m);
            }
        }

        // 4. Display name (fuzzy)
        if let Some(dn) = &app_id.display_name {
            if let Some(m) = self.check_display_name(dn, app_id) {
                return Some(m);
            }
        }

        None
    }
}

impl Default for AppSignatureStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use bb_common::enums::Platform;

    use super::*;

    fn make_app_id(platform: Platform) -> AppIdentifier {
        AppIdentifier::empty(platform)
    }

    fn test_signature() -> AppSignatureSummary {
        AppSignatureSummary {
            public_id: Uuid::nil(),
            name: "Bet365".to_string(),
            package_names: vec!["com.bet365.sportsbook".to_string()],
            executable_names: vec!["bet365.exe".to_string()],
            cert_hashes: vec!["sha256:abc123def456".to_string()],
            display_name_patterns: vec!["bet365".to_string()],
            platforms: vec!["windows".to_string()],
            category: "sports_betting".to_string(),
            confidence: 0.85,
        }
    }

    fn test_store() -> AppSignatureStore {
        AppSignatureStore::from_summaries(vec![test_signature()])
    }

    // --- Exact package name matching ---

    #[test]
    fn exact_package_name_match() {
        let store = test_store();
        let mut app = make_app_id(Platform::Windows);
        app.package_name = Some("com.bet365.sportsbook".to_string());
        let result = store.check_package_name("com.bet365.sportsbook", &app);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.match_type, AppMatchType::ExactPackage);
        assert_eq!(m.confidence, 1.0);
    }

    #[test]
    fn package_name_case_insensitive() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_package_name("COM.BET365.SPORTSBOOK", &app);
        assert!(result.is_some());
    }

    #[test]
    fn package_name_no_match() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_package_name("com.google.chrome", &app);
        assert!(result.is_none());
    }

    // --- Exact executable name matching ---

    #[test]
    fn exact_executable_match() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_executable("bet365.exe", &app);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.match_type, AppMatchType::ExactExecutable);
    }

    #[test]
    fn executable_case_insensitive() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_executable("BET365.EXE", &app);
        assert!(result.is_some());
    }

    #[test]
    fn executable_no_match() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_executable("chrome.exe", &app);
        assert!(result.is_none());
    }

    // --- Cert hash matching ---

    #[test]
    fn exact_cert_hash_match() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_cert_hash("sha256:abc123def456", &app);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.match_type, AppMatchType::CertHash);
    }

    #[test]
    fn cert_hash_case_insensitive() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_cert_hash("SHA256:ABC123DEF456", &app);
        assert!(result.is_some());
    }

    #[test]
    fn cert_hash_no_match() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_cert_hash("sha256:000000", &app);
        assert!(result.is_none());
    }

    // --- Fuzzy display name matching ---

    #[test]
    fn fuzzy_display_name_exact() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_display_name("bet365", &app);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.match_type, AppMatchType::FuzzyDisplayName);
        assert!(m.confidence >= 0.85);
    }

    #[test]
    fn fuzzy_display_name_close_match() {
        // "Bet 365 Sportsbook" vs pattern "bet365"
        // These are quite different strings, so we use a signature with a lower confidence
        // and a closer display name pattern.
        let sig = AppSignatureSummary {
            public_id: Uuid::nil(),
            name: "Bet365".to_string(),
            package_names: vec![],
            executable_names: vec![],
            cert_hashes: vec![],
            display_name_patterns: vec!["Bet 365 Sportsbook".to_string()],
            platforms: vec!["windows".to_string()],
            category: "sports_betting".to_string(),
            confidence: 0.80,
        };
        let store = AppSignatureStore::from_summaries(vec![sig]);
        let app = make_app_id(Platform::Windows);
        let result = store.check_display_name("Bet 365 Sportsbook", &app);
        assert!(result.is_some());
    }

    #[test]
    fn fuzzy_display_name_no_false_positive() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        // "beta365tool" should NOT match "bet365" at 0.85 threshold
        let result = store.check_display_name("beta365tool", &app);
        // Normalized Levenshtein of "beta365tool" vs "bet365" is low enough to reject
        assert!(result.is_none());
    }

    #[test]
    fn display_name_case_insensitive() {
        let store = test_store();
        let app = make_app_id(Platform::Windows);
        let result = store.check_display_name("BET365", &app);
        assert!(result.is_some());
    }

    // --- check_app integration ---

    #[test]
    fn check_app_prefers_cert_hash() {
        let store = test_store();
        let mut app = make_app_id(Platform::Windows);
        app.cert_hash = Some("sha256:abc123def456".to_string());
        app.package_name = Some("com.bet365.sportsbook".to_string());
        let result = store.check_app(&app);
        assert!(result.is_some());
        assert_eq!(result.unwrap().match_type, AppMatchType::CertHash);
    }

    #[test]
    fn check_app_falls_through_to_package() {
        let store = test_store();
        let mut app = make_app_id(Platform::Windows);
        app.cert_hash = Some("sha256:unknown".to_string());
        app.package_name = Some("com.bet365.sportsbook".to_string());
        let result = store.check_app(&app);
        assert!(result.is_some());
        assert_eq!(result.unwrap().match_type, AppMatchType::ExactPackage);
    }

    #[test]
    fn check_app_falls_through_to_executable() {
        let store = test_store();
        let mut app = make_app_id(Platform::Windows);
        app.executable_name = Some("bet365.exe".to_string());
        let result = store.check_app(&app);
        assert!(result.is_some());
        assert_eq!(result.unwrap().match_type, AppMatchType::ExactExecutable);
    }

    #[test]
    fn check_app_falls_through_to_display_name() {
        let store = test_store();
        let mut app = make_app_id(Platform::Windows);
        app.display_name = Some("bet365".to_string());
        let result = store.check_app(&app);
        assert!(result.is_some());
        assert_eq!(result.unwrap().match_type, AppMatchType::FuzzyDisplayName);
    }

    #[test]
    fn check_app_no_match() {
        let store = test_store();
        let mut app = make_app_id(Platform::Windows);
        app.display_name = Some("Google Chrome".to_string());
        app.executable_name = Some("chrome.exe".to_string());
        let result = store.check_app(&app);
        assert!(result.is_none());
    }

    // --- Empty store ---

    #[test]
    fn empty_store_returns_none() {
        let store = AppSignatureStore::new();
        assert!(store.is_empty());
        let app = make_app_id(Platform::Windows);
        assert!(store.check_package_name("anything", &app).is_none());
        assert!(store.check_executable("anything", &app).is_none());
        assert!(store.check_cert_hash("anything", &app).is_none());
        assert!(store.check_display_name("anything", &app).is_none());

        let mut full_app = make_app_id(Platform::Windows);
        full_app.package_name = Some("com.test".to_string());
        assert!(store.check_app(&full_app).is_none());
    }

    // --- Multiple signatures ---

    #[test]
    fn multiple_signatures_indexed() {
        let sigs = vec![
            AppSignatureSummary {
                public_id: Uuid::from_u128(1),
                name: "Bet365".to_string(),
                package_names: vec!["com.bet365.app".to_string()],
                executable_names: vec!["bet365.exe".to_string()],
                cert_hashes: vec![],
                display_name_patterns: vec!["bet365".to_string()],
                platforms: vec!["windows".to_string()],
                category: "sports_betting".to_string(),
                confidence: 0.85,
            },
            AppSignatureSummary {
                public_id: Uuid::from_u128(2),
                name: "PokerStars".to_string(),
                package_names: vec!["com.pokerstars.app".to_string()],
                executable_names: vec!["pokerstars.exe".to_string()],
                cert_hashes: vec![],
                display_name_patterns: vec!["pokerstars".to_string()],
                platforms: vec!["windows".to_string()],
                category: "poker".to_string(),
                confidence: 0.85,
            },
        ];
        let store = AppSignatureStore::from_summaries(sigs);
        assert_eq!(store.len(), 2);

        let app = make_app_id(Platform::Windows);

        let r1 = store.check_package_name("com.bet365.app", &app);
        assert!(r1.is_some());
        assert_eq!(r1.unwrap().signature_name, "Bet365");

        let r2 = store.check_package_name("com.pokerstars.app", &app);
        assert!(r2.is_some());
        assert_eq!(r2.unwrap().signature_name, "PokerStars");
    }
}
