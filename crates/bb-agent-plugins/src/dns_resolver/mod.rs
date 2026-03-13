pub mod handler;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use hickory_server::ServerFuture;
use tokio::net::UdpSocket;
use tracing::{error, info};

use crate::blocklist::Blocklist;
use crate::traits::{BlockingPlugin, DnsBlockingPlugin};
use crate::types::{BlockDecision, BlockingLayer, PluginConfig, PluginError, PluginHealth};

use handler::{BlockResponse, BlockingDnsHandler};

const DEFAULT_UPSTREAM_1: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53));
const DEFAULT_UPSTREAM_2: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(1, 1, 1, 1), 53));
const DEFAULT_LISTEN_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 53));

/// DNS query counters for health reporting.
#[derive(Debug, Default)]
pub struct DnsMetrics {
    pub queries_total: AtomicU64,
    pub queries_blocked: AtomicU64,
    pub queries_forwarded: AtomicU64,
    pub upstream_errors: AtomicU64,
}

/// A local DNS resolver that intercepts queries, checks the blocklist,
/// and either returns NXDOMAIN or forwards to upstream.
pub struct DnsResolverPlugin {
    blocklist: Option<Arc<Blocklist>>,
    upstream_servers: Vec<SocketAddr>,
    listen_addr: SocketAddr,
    block_response: BlockResponse,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    active: bool,
    pub metrics: Arc<DnsMetrics>,
}

impl DnsResolverPlugin {
    pub fn new() -> Self {
        Self {
            blocklist: None,
            upstream_servers: vec![DEFAULT_UPSTREAM_1, DEFAULT_UPSTREAM_2],
            listen_addr: DEFAULT_LISTEN_ADDR,
            block_response: BlockResponse::NxDomain,
            server_handle: None,
            active: false,
            metrics: Arc::new(DnsMetrics::default()),
        }
    }
}

impl Default for DnsResolverPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DnsResolverPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DnsResolverPlugin")
            .field("listen_addr", &self.listen_addr)
            .field("upstream_servers", &self.upstream_servers)
            .field("active", &self.active)
            .finish()
    }
}

impl BlockingPlugin for DnsResolverPlugin {
    fn id(&self) -> &str {
        "dns.resolver"
    }

    fn name(&self) -> &str {
        "DNS Resolver"
    }

    fn layer(&self) -> BlockingLayer {
        BlockingLayer::Dns
    }

    fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError> {
        // Parse upstream servers from config
        if let Some(servers) = config.settings.get("upstream_servers")
            && let Some(arr) = servers.as_array()
        {
            let mut parsed = Vec::new();
            for s in arr {
                if let Some(addr_str) = s.as_str() {
                    let addr: SocketAddr = addr_str.parse().map_err(|e| {
                        PluginError::ConfigError(format!(
                            "Invalid upstream server address '{addr_str}': {e}"
                        ))
                    })?;
                    parsed.push(addr);
                }
            }
            if !parsed.is_empty() {
                self.upstream_servers = parsed;
            }
        }

        // Parse listen address
        if let Some(addr) = config.settings.get("listen_addr")
            && let Some(addr_str) = addr.as_str()
        {
            self.listen_addr = addr_str.parse().map_err(|e| {
                PluginError::ConfigError(format!(
                    "Invalid listen address '{addr_str}': {e}"
                ))
            })?;
        }

        // Parse listen port (can override just the port)
        if let Some(port) = config.settings.get("listen_port")
            && let Some(p) = port.as_u64()
        {
            let p = u16::try_from(p).map_err(|_| {
                PluginError::ConfigError(format!("Invalid port number: {p}"))
            })?;
            self.listen_addr.set_port(p);
        }

        // Parse block response type
        if let Some(br) = config.settings.get("block_response")
            && let Some(br_str) = br.as_str()
        {
            self.block_response = match br_str {
                "nxdomain" => BlockResponse::NxDomain,
                "zero_ip" => BlockResponse::ZeroIp,
                other => {
                    return Err(PluginError::ConfigError(format!(
                        "Unknown block_response: '{other}'. Expected 'nxdomain' or 'zero_ip'"
                    )));
                }
            };
        }

        info!(
            listen_addr = %self.listen_addr,
            upstream_count = self.upstream_servers.len(),
            "DNS resolver plugin initialized"
        );

        Ok(())
    }

    fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        let bl = Arc::new(blocklist.clone());
        self.blocklist = Some(Arc::clone(&bl));

        let handler = BlockingDnsHandler::new(bl, &self.upstream_servers, self.block_response);
        let listen_addr = self.listen_addr;

        let handle = tokio::spawn(async move {
            let socket = match UdpSocket::bind(listen_addr).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, addr = %listen_addr, "Failed to bind DNS UDP socket");
                    return;
                }
            };

            info!(addr = %listen_addr, "DNS server listening");

            let mut server = ServerFuture::new(handler);
            server.register_socket(socket);

            if let Err(e) = server.block_until_done().await {
                error!(error = %e, "DNS server exited with error");
            }
        });

        self.server_handle = Some(handle);
        self.active = true;

        info!("DNS resolver plugin activated");
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), PluginError> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        self.active = false;
        info!("DNS resolver plugin deactivated");
        Ok(())
    }

    fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        self.blocklist = Some(Arc::new(blocklist.clone()));
        // Note: The running DNS handler still uses the old Arc.
        // For a full hot-swap, we'd need shared state (ArcSwap).
        // For now, the handler's blocklist is set at activation time.
        // A full restart is needed for blocklist updates to take effect in the DNS server.
        info!(version = blocklist.version, "DNS resolver blocklist updated");
        Ok(())
    }

    fn health_check(&self) -> Result<PluginHealth, PluginError> {
        if !self.active {
            return Ok(PluginHealth::degraded("DNS resolver is not active"));
        }

        if let Some(handle) = &self.server_handle
            && handle.is_finished()
        {
            return Ok(PluginHealth::degraded(
                "DNS server task has exited unexpectedly",
            ));
        }

        let mut health = PluginHealth::ok();
        health.details.insert(
            "queries_total".into(),
            self.metrics
                .queries_total
                .load(Ordering::Relaxed)
                .to_string(),
        );
        health.details.insert(
            "queries_blocked".into(),
            self.metrics
                .queries_blocked
                .load(Ordering::Relaxed)
                .to_string(),
        );
        Ok(health)
    }
}

impl DnsBlockingPlugin for DnsResolverPlugin {
    fn check_domain(&self, domain: &str) -> BlockDecision {
        match &self.blocklist {
            Some(bl) if bl.is_blocked(domain) => BlockDecision::Block {
                reason: format!("Domain '{domain}' is in the blocklist"),
            },
            Some(_) => BlockDecision::Allow,
            None => BlockDecision::Abstain,
        }
    }

    fn handle_dns_query(&self, _query: &[u8]) -> Option<Vec<u8>> {
        // This plugin uses hickory-dns server, not raw packet handling
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_domain_blocked() {
        let mut plugin = DnsResolverPlugin::new();
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        plugin.blocklist = Some(Arc::new(bl));

        let decision = plugin.check_domain("bet365.com");
        assert!(decision.is_blocked());
    }

    #[test]
    fn test_check_domain_allowed() {
        let mut plugin = DnsResolverPlugin::new();
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        plugin.blocklist = Some(Arc::new(bl));

        let decision = plugin.check_domain("google.com");
        assert!(!decision.is_blocked());
        assert_eq!(decision, BlockDecision::Allow);
    }

    #[test]
    fn test_check_domain_no_blocklist() {
        let plugin = DnsResolverPlugin::new();
        let decision = plugin.check_domain("anything.com");
        assert_eq!(decision, BlockDecision::Abstain);
    }

    #[test]
    fn test_check_subdomain_blocked() {
        let mut plugin = DnsResolverPlugin::new();
        let mut bl = Blocklist::new(1);
        bl.add_entry("bet365.com");
        plugin.blocklist = Some(Arc::new(bl));

        assert!(plugin.check_domain("www.bet365.com").is_blocked());
        assert!(plugin.check_domain("sub.deep.bet365.com").is_blocked());
    }

    #[test]
    fn test_init_parses_config() {
        let mut plugin = DnsResolverPlugin::new();
        let mut config = PluginConfig::default();
        config.settings.insert(
            "upstream_servers".into(),
            serde_json::json!(["9.9.9.9:53", "149.112.112.112:53"]),
        );
        config
            .settings
            .insert("listen_port".into(), serde_json::json!(15353));
        config
            .settings
            .insert("block_response".into(), serde_json::json!("zero_ip"));

        plugin.init(&config).expect("init should succeed");

        assert_eq!(plugin.upstream_servers.len(), 2);
        assert_eq!(plugin.listen_addr.port(), 15353);
        assert_eq!(plugin.block_response, BlockResponse::ZeroIp);
    }

    #[test]
    fn test_health_check_inactive() {
        let plugin = DnsResolverPlugin::new();
        let health = plugin.health_check().expect("health check");
        assert!(!health.healthy);
    }
}
