use std::net::IpAddr;
use std::sync::Arc;

use bb_common::models::bypass_detection::TorInfo;
use bb_common::models::tor_exit_nodes::TorExitNodeList;
use tokio::sync::RwLock;

use super::known_processes::TOR_PROCESS_NAMES;
use super::traits::{BypassDetectionError, ProcessScanner};

/// Orchestrates Tor detection by combining process scanning with
/// exit-node list lookups.
pub struct TorDetector {
    process_scanner: Box<dyn ProcessScanner>,
    exit_nodes: Arc<RwLock<Option<TorExitNodeList>>>,
}

impl TorDetector {
    pub fn new(
        process_scanner: Box<dyn ProcessScanner>,
        exit_nodes: Arc<RwLock<Option<TorExitNodeList>>>,
    ) -> Self {
        Self {
            process_scanner,
            exit_nodes,
        }
    }

    /// Detect whether Tor is running by scanning for known process names.
    pub async fn detect(&self) -> Result<TorInfo, BypassDetectionError> {
        let found = self
            .process_scanner
            .scan_for_processes(TOR_PROCESS_NAMES)
            .await?;
        Ok(TorInfo {
            process_detected: !found.is_empty(),
            exit_node_match: false,
        })
    }

    /// Replace the cached exit-node list with a fresh one.
    pub async fn update_exit_nodes(&self, list: TorExitNodeList) {
        let mut guard = self.exit_nodes.write().await;
        *guard = Some(list);
    }

    /// Check whether the given IP address is a known Tor exit node.
    pub async fn is_exit_node(&self, ip: &IpAddr) -> bool {
        let guard = self.exit_nodes.read().await;
        guard.as_ref().is_some_and(|list| list.contains(ip))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;

    struct MockProcessScanner {
        found: Vec<String>,
    }

    #[async_trait]
    impl ProcessScanner for MockProcessScanner {
        async fn scan_for_processes(
            &self,
            _known_names: &[&str],
        ) -> Result<Vec<String>, BypassDetectionError> {
            Ok(self.found.clone())
        }
    }

    #[tokio::test]
    async fn no_tor_detected() {
        let detector = TorDetector::new(
            Box::new(MockProcessScanner { found: vec![] }),
            Arc::new(RwLock::new(None)),
        );
        let info = detector.detect().await.unwrap();
        assert!(!info.process_detected);
        assert!(!info.exit_node_match);
    }

    #[tokio::test]
    async fn tor_process_detected() {
        let detector = TorDetector::new(
            Box::new(MockProcessScanner {
                found: vec!["tor".to_string()],
            }),
            Arc::new(RwLock::new(None)),
        );
        let info = detector.detect().await.unwrap();
        assert!(info.process_detected);
    }

    #[tokio::test]
    async fn exit_node_match() {
        let detector = TorDetector::new(
            Box::new(MockProcessScanner { found: vec![] }),
            Arc::new(RwLock::new(None)),
        );

        let now = Utc::now();
        let later = now + chrono::Duration::hours(1);
        let list = TorExitNodeList::parse_from_csv("1.2.3.4\n5.6.7.8\n", now, later);
        detector.update_exit_nodes(list).await;

        assert!(detector.is_exit_node(&"1.2.3.4".parse().unwrap()).await);
        assert!(!detector.is_exit_node(&"9.9.9.9".parse().unwrap()).await);
    }

    #[tokio::test]
    async fn no_exit_nodes_loaded() {
        let detector = TorDetector::new(
            Box::new(MockProcessScanner { found: vec![] }),
            Arc::new(RwLock::new(None)),
        );
        assert!(!detector.is_exit_node(&"1.2.3.4".parse().unwrap()).await);
    }
}
