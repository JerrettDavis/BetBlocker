use std::collections::HashSet;
use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorExitNodeList {
    pub nodes: HashSet<IpAddr>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl TorExitNodeList {
    /// Check whether the given IP is a known Tor exit node.
    #[must_use]
    pub fn contains(&self, ip: &IpAddr) -> bool {
        self.nodes.contains(ip)
    }

    /// Parse a newline-delimited list of IP addresses (CSV-style, one per line).
    /// Lines that fail to parse are silently skipped.
    #[must_use]
    pub fn parse_from_csv(
        data: &str,
        fetched_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        let nodes = data
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return None;
                }
                trimmed.parse::<IpAddr>().ok()
            })
            .collect();

        Self {
            nodes,
            fetched_at,
            expires_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_from_csv_basic() {
        let data = "1.2.3.4\n5.6.7.8\n";
        let now = Utc::now();
        let later = now + chrono::Duration::hours(1);
        let list = TorExitNodeList::parse_from_csv(data, now, later);
        assert_eq!(list.nodes.len(), 2);
        assert!(list.contains(&"1.2.3.4".parse().unwrap()));
        assert!(list.contains(&"5.6.7.8".parse().unwrap()));
        assert!(!list.contains(&"9.9.9.9".parse().unwrap()));
    }

    #[test]
    fn parse_from_csv_skips_comments_and_blanks() {
        let data = "# comment\n\n1.2.3.4\n  \n# another comment\n::1\n";
        let now = Utc::now();
        let later = now + chrono::Duration::hours(1);
        let list = TorExitNodeList::parse_from_csv(data, now, later);
        assert_eq!(list.nodes.len(), 2);
        assert!(list.contains(&"1.2.3.4".parse().unwrap()));
        assert!(list.contains(&"::1".parse().unwrap()));
    }

    #[test]
    fn parse_from_csv_skips_invalid() {
        let data = "1.2.3.4\nnot_an_ip\n5.6.7.8\n";
        let now = Utc::now();
        let later = now + chrono::Duration::hours(1);
        let list = TorExitNodeList::parse_from_csv(data, now, later);
        assert_eq!(list.nodes.len(), 2);
    }

    #[test]
    fn tor_exit_node_list_roundtrips_json() {
        let data = "1.2.3.4\n::1\n";
        let now = Utc::now();
        let later = now + chrono::Duration::hours(1);
        let list = TorExitNodeList::parse_from_csv(data, now, later);
        let json = serde_json::to_string(&list).unwrap();
        let back: TorExitNodeList = serde_json::from_str(&json).unwrap();
        assert_eq!(list.nodes.len(), back.nodes.len());
        assert!(back.contains(&"1.2.3.4".parse().unwrap()));
    }
}
