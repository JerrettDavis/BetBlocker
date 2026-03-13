use std::io::Write;
use std::path::{Path, PathBuf};

use super::Blocklist;

/// Persists a `Blocklist` to disk for crash recovery and offline startup.
#[derive(Debug)]
pub struct BlocklistCache {
    path: PathBuf,
}

impl BlocklistCache {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Save the blocklist to a newline-delimited file with a version header.
    /// Writes to a temp file first, then atomically renames for crash safety.
    pub fn save(&self, blocklist: &Blocklist) -> Result<(), std::io::Error> {
        let tmp_path = self.path.with_extension("tmp");

        let mut file = std::fs::File::create(&tmp_path)?;
        writeln!(file, "# version: {}", blocklist.version)?;

        // Write exact domains
        let mut domains: Vec<&str> = blocklist.exact_domains().collect();
        domains.sort_unstable();
        for domain in domains {
            writeln!(file, "{domain}")?;
        }

        // Write wildcard suffixes (convert back from ".suffix" to "*.suffix")
        let mut wildcards: Vec<&str> = blocklist
            .wildcard_suffixes()
            .iter()
            .map(String::as_str)
            .collect();
        wildcards.sort_unstable();
        for suffix in wildcards {
            // suffix is ".example.com", convert to "*.example.com"
            writeln!(file, "*{suffix}")?;
        }

        file.flush()?;
        drop(file);

        // Atomic rename
        std::fs::rename(&tmp_path, &self.path)?;

        Ok(())
    }

    /// Load the blocklist from the cache file.
    pub fn load(&self) -> Result<Blocklist, std::io::Error> {
        let content = std::fs::read_to_string(&self.path)?;

        // Parse version from first comment line: "# version: <N>"
        let mut version: i64 = 0;
        for line in content.lines() {
            let line = line.trim();
            if let Some(ver_str) = line.strip_prefix("# version:") {
                if let Ok(v) = ver_str.trim().parse::<i64>() {
                    version = v;
                }
                break;
            }
            // Only look at the first non-empty line
            if !line.is_empty() {
                break;
            }
        }

        Ok(Blocklist::from_str_content(&content, version))
    }

    /// Check if the cache file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Apply a delta update to a blocklist: add new entries, remove old ones,
    /// bump the version, and persist.
    pub fn apply_delta(
        &self,
        blocklist: &mut Blocklist,
        added: &[String],
        removed: &[String],
        new_version: i64,
    ) -> Result<(), std::io::Error> {
        for entry in added {
            blocklist.add_entry(entry);
        }
        for entry in removed {
            blocklist.remove_entry(entry);
        }
        blocklist.version = new_version;
        self.save(blocklist)
    }

    /// Returns the path of the cache file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_roundtrip() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = BlocklistCache::new(dir.path().join("blocklist.cache"));

        let mut bl = Blocklist::new(42);
        bl.add_entry("bet365.com");
        bl.add_entry("pokerstars.com");
        bl.add_entry("*.gambling-site.com");

        cache.save(&bl).expect("save");
        assert!(cache.exists());

        let loaded = cache.load().expect("load");
        assert_eq!(loaded.version, 42);
        assert_eq!(loaded.len(), 3);
        assert!(loaded.is_blocked("bet365.com"));
        assert!(loaded.is_blocked("pokerstars.com"));
        assert!(loaded.is_blocked("sub.gambling-site.com"));
    }

    #[test]
    fn test_delta_application() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = BlocklistCache::new(dir.path().join("blocklist.cache"));

        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        bl.add_entry("old-site.com");
        cache.save(&bl).expect("save");

        cache
            .apply_delta(
                &mut bl,
                &["newsite.com".to_string()],
                &["old-site.com".to_string()],
                2,
            )
            .expect("delta");

        assert_eq!(bl.version, 2);
        assert!(bl.is_blocked("bet365.com"));
        assert!(bl.is_blocked("newsite.com"));
        assert!(!bl.is_blocked("old-site.com"));

        // Verify persistence
        let loaded = cache.load().expect("load");
        assert_eq!(loaded.version, 2);
        assert!(loaded.is_blocked("newsite.com"));
        assert!(!loaded.is_blocked("old-site.com"));
    }

    #[test]
    fn test_cache_not_exists() {
        let cache = BlocklistCache::new(PathBuf::from("/nonexistent/path/blocklist.cache"));
        assert!(!cache.exists());
    }
}
