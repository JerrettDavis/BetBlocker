use bb_common::models::bypass_detection::ProxyInfo;

use super::traits::{BypassDetectionError, ProxyConfigMonitor};

/// Orchestrates proxy configuration detection.
pub struct ProxyDetector {
    config_monitor: Box<dyn ProxyConfigMonitor>,
}

impl ProxyDetector {
    pub fn new(config_monitor: Box<dyn ProxyConfigMonitor>) -> Self {
        Self { config_monitor }
    }

    /// Detect whether the system has a proxy configured.
    pub async fn detect(&self) -> Result<Option<ProxyInfo>, BypassDetectionError> {
        self.config_monitor.detect_proxy_config().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bb_common::models::bypass_detection::{ProxySource, ProxyType};

    struct MockProxyMonitor {
        result: Option<ProxyInfo>,
    }

    #[async_trait]
    impl ProxyConfigMonitor for MockProxyMonitor {
        async fn detect_proxy_config(&self) -> Result<Option<ProxyInfo>, BypassDetectionError> {
            Ok(self.result.clone())
        }
    }

    #[tokio::test]
    async fn no_proxy() {
        let detector = ProxyDetector::new(Box::new(MockProxyMonitor { result: None }));
        let result = detector.detect().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn http_proxy_detected() {
        let detector = ProxyDetector::new(Box::new(MockProxyMonitor {
            result: Some(ProxyInfo {
                proxy_type: ProxyType::Http,
                address: "http://proxy.example.com:8080".to_string(),
                source: ProxySource::SystemSettings,
            }),
        }));
        let result = detector.detect().await.unwrap().unwrap();
        assert_eq!(result.proxy_type, ProxyType::Http);
        assert_eq!(result.address, "http://proxy.example.com:8080");
    }

    #[tokio::test]
    async fn socks_proxy_detected() {
        let detector = ProxyDetector::new(Box::new(MockProxyMonitor {
            result: Some(ProxyInfo {
                proxy_type: ProxyType::Socks5,
                address: "127.0.0.1:1080".to_string(),
                source: ProxySource::EnvironmentVariable,
            }),
        }));
        let result = detector.detect().await.unwrap().unwrap();
        assert_eq!(result.proxy_type, ProxyType::Socks5);
        assert_eq!(result.source, ProxySource::EnvironmentVariable);
    }
}
