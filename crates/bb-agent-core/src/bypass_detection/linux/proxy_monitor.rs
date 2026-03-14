use async_trait::async_trait;
use bb_common::models::bypass_detection::{ProxyInfo, ProxySource, ProxyType};

use crate::bypass_detection::traits::{BypassDetectionError, ProxyConfigMonitor};

/// Linux proxy configuration monitor that reads environment variables
/// (`http_proxy`, `https_proxy`, `all_proxy` and their uppercase equivalents)
/// to detect proxy settings.
///
/// This implementation is cross-platform since it only reads env vars.
pub struct LinuxProxyMonitor;

impl LinuxProxyMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxProxyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Environment variable names to check, in priority order.
const PROXY_ENV_VARS: &[&str] = &[
    "http_proxy",
    "HTTP_PROXY",
    "https_proxy",
    "HTTPS_PROXY",
    "all_proxy",
    "ALL_PROXY",
];

/// Parse a proxy URL string into a `ProxyInfo`.
fn parse_proxy_url(url: &str, env_var: &str) -> ProxyInfo {
    let proxy_type = infer_proxy_type(url, env_var);
    ProxyInfo {
        proxy_type,
        address: url.to_string(),
        source: ProxySource::EnvironmentVariable,
    }
}

/// Infer the proxy type from the URL scheme and the env var name.
fn infer_proxy_type(url: &str, env_var: &str) -> ProxyType {
    let lower_url = url.to_lowercase();

    if lower_url.starts_with("socks5://") || lower_url.starts_with("socks5h://") {
        ProxyType::Socks5
    } else if lower_url.starts_with("socks4://") || lower_url.starts_with("socks4a://") {
        ProxyType::Socks4
    } else if lower_url.starts_with("https://") {
        ProxyType::Https
    } else if lower_url.starts_with("http://") {
        ProxyType::Http
    } else {
        // Fall back to inferring from env var name.
        let lower_var = env_var.to_lowercase();
        if lower_var.starts_with("https") {
            ProxyType::Https
        } else {
            ProxyType::Http
        }
    }
}

#[async_trait]
impl ProxyConfigMonitor for LinuxProxyMonitor {
    async fn detect_proxy_config(&self) -> Result<Option<ProxyInfo>, BypassDetectionError> {
        for &var_name in PROXY_ENV_VARS {
            if let Ok(val) = std::env::var(var_name) {
                let val = val.trim().to_string();
                if !val.is_empty() {
                    return Ok(Some(parse_proxy_url(&val, var_name)));
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_proxy_url tests ───────────────────────────────────────

    #[test]
    fn parse_http_proxy_url() {
        let info = parse_proxy_url("http://proxy.example.com:3128", "http_proxy");
        assert_eq!(info.proxy_type, ProxyType::Http);
        assert_eq!(info.address, "http://proxy.example.com:3128");
        assert_eq!(info.source, ProxySource::EnvironmentVariable);
    }

    #[test]
    fn parse_https_proxy_url() {
        let info = parse_proxy_url("https://secure-proxy:443", "HTTPS_PROXY");
        assert_eq!(info.proxy_type, ProxyType::Https);
        assert_eq!(info.address, "https://secure-proxy:443");
    }

    #[test]
    fn parse_socks5_proxy_url() {
        let info = parse_proxy_url("socks5://127.0.0.1:1080", "ALL_PROXY");
        assert_eq!(info.proxy_type, ProxyType::Socks5);
        assert_eq!(info.address, "socks5://127.0.0.1:1080");
    }

    #[test]
    fn parse_socks4_proxy_url() {
        let info = parse_proxy_url("socks4://10.0.0.1:1080", "all_proxy");
        assert_eq!(info.proxy_type, ProxyType::Socks4);
    }

    #[test]
    fn parse_bare_address_uses_env_var_hint() {
        let info = parse_proxy_url("10.0.0.1:8080", "https_proxy");
        assert_eq!(info.proxy_type, ProxyType::Https);
        assert_eq!(info.address, "10.0.0.1:8080");

        let info2 = parse_proxy_url("10.0.0.1:8080", "http_proxy");
        assert_eq!(info2.proxy_type, ProxyType::Http);
    }

    // ── infer_proxy_type tests ──────────────────────────────────────

    #[test]
    fn infer_http_from_scheme() {
        assert_eq!(
            infer_proxy_type("http://proxy:8080", "http_proxy"),
            ProxyType::Http
        );
    }

    #[test]
    fn infer_https_from_scheme() {
        assert_eq!(
            infer_proxy_type("https://proxy:8080", "https_proxy"),
            ProxyType::Https
        );
    }

    #[test]
    fn infer_socks5_from_scheme() {
        assert_eq!(
            infer_proxy_type("socks5://127.0.0.1:1080", "all_proxy"),
            ProxyType::Socks5
        );
    }

    #[test]
    fn infer_socks5h_from_scheme() {
        assert_eq!(
            infer_proxy_type("socks5h://127.0.0.1:1080", "all_proxy"),
            ProxyType::Socks5
        );
    }

    #[test]
    fn infer_socks4_from_scheme() {
        assert_eq!(
            infer_proxy_type("socks4://127.0.0.1:1080", "all_proxy"),
            ProxyType::Socks4
        );
    }

    #[test]
    fn infer_socks4a_from_scheme() {
        assert_eq!(
            infer_proxy_type("socks4a://127.0.0.1:1080", "all_proxy"),
            ProxyType::Socks4
        );
    }

    #[test]
    fn infer_from_env_var_name_fallback() {
        assert_eq!(
            infer_proxy_type("127.0.0.1:8080", "https_proxy"),
            ProxyType::Https
        );
        assert_eq!(
            infer_proxy_type("127.0.0.1:8080", "http_proxy"),
            ProxyType::Http
        );
    }

    #[test]
    fn infer_case_insensitive_scheme() {
        assert_eq!(
            infer_proxy_type("HTTP://proxy:8080", "all_proxy"),
            ProxyType::Http
        );
        assert_eq!(
            infer_proxy_type("SOCKS5://proxy:1080", "all_proxy"),
            ProxyType::Socks5
        );
    }

    // ── env var list ────────────────────────────────────────────────

    #[test]
    fn proxy_env_vars_contains_expected_entries() {
        assert!(PROXY_ENV_VARS.contains(&"http_proxy"));
        assert!(PROXY_ENV_VARS.contains(&"HTTP_PROXY"));
        assert!(PROXY_ENV_VARS.contains(&"https_proxy"));
        assert!(PROXY_ENV_VARS.contains(&"HTTPS_PROXY"));
        assert!(PROXY_ENV_VARS.contains(&"all_proxy"));
        assert!(PROXY_ENV_VARS.contains(&"ALL_PROXY"));
    }

    // ── source is always EnvironmentVariable ────────────────────────

    #[test]
    fn source_is_always_environment_variable() {
        let info = parse_proxy_url("http://x:80", "http_proxy");
        assert_eq!(info.source, ProxySource::EnvironmentVariable);
    }
}
