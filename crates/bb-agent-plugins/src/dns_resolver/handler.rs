use std::net::SocketAddr;
use std::sync::Arc;

use hickory_resolver::TokioResolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::xfer::Protocol;
use hickory_server::authority::MessageResponseBuilder;
use hickory_server::proto::op::{Header, ResponseCode};
use hickory_server::proto::rr::{Name, RData, Record, rdata::A};
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use tracing::{debug, warn};

use crate::blocklist::Blocklist;

/// How the DNS resolver responds to blocked domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockResponse {
    /// Return NXDOMAIN (domain does not exist). Default.
    NxDomain,
    /// Return 0.0.0.0 as the A record (connection will time out).
    ZeroIp,
}

/// Handles incoming DNS requests: checks blocklist, forwards or blocks.
pub struct BlockingDnsHandler {
    blocklist: Arc<Blocklist>,
    upstream: TokioResolver,
    block_response: BlockResponse,
}

impl BlockingDnsHandler {
    pub fn new(
        blocklist: Arc<Blocklist>,
        upstream_servers: &[SocketAddr],
        block_response: BlockResponse,
    ) -> Self {
        // Build resolver config pointing to upstream DNS servers
        let mut resolver_config = ResolverConfig::new();
        for addr in upstream_servers {
            resolver_config.add_name_server(NameServerConfig::new(*addr, Protocol::Udp));
        }
        let upstream =
            TokioResolver::builder_with_config(resolver_config, TokioConnectionProvider::default())
                .build();

        Self {
            blocklist,
            upstream,
            block_response,
        }
    }

    /// Update the blocklist atomically (Arc swap).
    pub fn update_blocklist(&mut self, blocklist: Arc<Blocklist>) {
        self.blocklist = blocklist;
    }

    fn servfail_header() -> ResponseInfo {
        let mut header = Header::new();
        header.set_response_code(ResponseCode::ServFail);
        header.into()
    }
}

#[async_trait::async_trait]
impl RequestHandler for BlockingDnsHandler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let info = match request.request_info() {
            Ok(info) => info,
            Err(e) => {
                warn!(error = %e, "Failed to parse DNS request info");
                return Self::servfail_header();
            }
        };
        let name = info.query.name();
        let query_type = info.query.query_type();
        let domain = name.to_string();
        let domain = domain.trim_end_matches('.');

        if self.blocklist.is_blocked(domain) {
            debug!(domain = %domain, "Blocked DNS query");

            let builder = MessageResponseBuilder::from_message_request(request);

            match self.block_response {
                BlockResponse::NxDomain => {
                    let response = builder.error_msg(request.header(), ResponseCode::NXDomain);
                    return response_handle
                        .send_response(response)
                        .await
                        .unwrap_or_else(|e| {
                            warn!(error = %e, "Failed to send NXDOMAIN response");
                            Self::servfail_header()
                        });
                }
                BlockResponse::ZeroIp => {
                    let name_parsed =
                        Name::from_str_relaxed(name.to_string().as_str()).unwrap_or_default();
                    let record = Record::from_rdata(name_parsed, 60, RData::A(A::new(0, 0, 0, 0)));

                    let response = builder.build(
                        *request.header(),
                        std::iter::once(&record),
                        std::iter::empty::<&Record>(),
                        std::iter::empty::<&Record>(),
                        std::iter::empty::<&Record>(),
                    );
                    return response_handle
                        .send_response(response)
                        .await
                        .unwrap_or_else(|e| {
                            warn!(error = %e, "Failed to send zero-IP response");
                            Self::servfail_header()
                        });
                }
            }
        }

        // Forward non-blocked queries to upstream
        debug!(domain = %domain, "Forwarding DNS query to upstream");
        match self.upstream.lookup(name, query_type).await {
            Ok(lookup) => {
                let builder = MessageResponseBuilder::from_message_request(request);
                let records: Vec<Record> = lookup.records().to_vec();
                let response = builder.build(
                    *request.header(),
                    records.iter(),
                    std::iter::empty::<&Record>(),
                    std::iter::empty::<&Record>(),
                    std::iter::empty::<&Record>(),
                );
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        warn!(error = %e, "Failed to send upstream response");
                        Self::servfail_header()
                    })
            }
            Err(e) => {
                warn!(domain = %domain, error = %e, "Upstream DNS lookup failed");
                let builder = MessageResponseBuilder::from_message_request(request);
                let response = builder.error_msg(request.header(), ResponseCode::ServFail);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        warn!(error = %e, "Failed to send SERVFAIL response");
                        Self::servfail_header()
                    })
            }
        }
    }
}
