use std::net::IpAddr;
use std::sync::Arc;

use bb_common::models::bypass_detection::TorInfo;
use bb_common::models::tor_exit_nodes::TorExitNodeList;
use chrono::Utc;
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

    /// Fetch a fresh Tor exit node list from the API and update the local cache.
    ///
    /// Expects the API to respond with a JSON object containing a `"nodes"`
    /// array of IP address strings, e.g.:
    /// ```json
    /// { "nodes": ["1.2.3.4", "::1"], "count": 2 }
    /// ```
    ///
    /// On any error the existing cached list is kept unchanged.
    pub async fn sync_exit_nodes(
        &self,
        api_base_url: &str,
        http: &reqwest::Client,
    ) -> Result<usize, BypassDetectionError> {
        let url = format!("{api_base_url}/v1/tor-exits");

        let response = http
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| BypassDetectionError::Other(e.to_string()))?
            .error_for_status()
            .map_err(|e| BypassDetectionError::Other(e.to_string()))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BypassDetectionError::Other(e.to_string()))?;

        let nodes_json = body
            .get("data")
            .and_then(|d| d.get("nodes"))
            .and_then(|n| n.as_array())
            .ok_or_else(|| {
                BypassDetectionError::Other("missing 'data.nodes' in response".into())
            })?;

        let nodes: std::collections::HashSet<IpAddr> = nodes_json
            .iter()
            .filter_map(|v| v.as_str()?.parse::<IpAddr>().ok())
            .collect();

        let count = nodes.len();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(6);

        let list = TorExitNodeList {
            nodes,
            fetched_at: now,
            expires_at,
        };

        self.update_exit_nodes(list).await;
        tracing::debug!(count, "synced Tor exit node list from API");

        Ok(count)
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

    // ── sync_exit_nodes ──────────────────────────────────────────────

    #[tokio::test]
    async fn sync_exit_nodes_parses_api_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/tor-exits"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "nodes": ["1.2.3.4", "5.6.7.8", "::1"],
                    "count": 3
                },
                "meta": { "timestamp": "2025-01-01T00:00:00Z" }
            })))
            .mount(&server)
            .await;

        let detector = TorDetector::new(
            Box::new(MockProcessScanner { found: vec![] }),
            Arc::new(RwLock::new(None)),
        );

        let http = reqwest::Client::new();
        let count = detector
            .sync_exit_nodes(&server.uri(), &http)
            .await
            .unwrap();

        assert_eq!(count, 3);
        assert!(detector.is_exit_node(&"1.2.3.4".parse().unwrap()).await);
        assert!(detector.is_exit_node(&"::1".parse().unwrap()).await);
        assert!(!detector.is_exit_node(&"9.9.9.9".parse().unwrap()).await);
    }

    #[tokio::test]
    async fn sync_exit_nodes_returns_error_on_bad_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/tor-exits"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let detector = TorDetector::new(
            Box::new(MockProcessScanner { found: vec![] }),
            Arc::new(RwLock::new(None)),
        );

        let http = reqwest::Client::new();
        let result = detector.sync_exit_nodes(&server.uri(), &http).await;
        assert!(result.is_err(), "should error on HTTP 500");
    }
}
