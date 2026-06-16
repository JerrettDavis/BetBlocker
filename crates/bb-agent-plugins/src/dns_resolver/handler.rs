use std::net::SocketAddr;
use std::sync::Arc;

use hickory_resolver::TokioResolver;
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolverConfig};
use hickory_server::net::runtime::{Time, TokioRuntimeProvider};
use hickory_server::proto::op::{
    Header, HeaderCounts, MessageType, Metadata, OpCode, ResponseCode,
};
use hickory_server::proto::rr::{Name, RData, Record, rdata::A};
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use hickory_server::zone_handler::MessageResponseBuilder;
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
        let name_servers = upstream_servers
            .iter()
            .map(|addr| {
                let mut connection = ConnectionConfig::udp();
                connection.port = addr.port();
                NameServerConfig::new(
                    addr.ip(),
                    true,
                    vec![connection],
                )
            })
            .collect();
        let resolver_config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
        let upstream = TokioResolver::builder_with_config(
            resolver_config,
            TokioRuntimeProvider::default(),
        )
        .build()
        .expect("upstream DNS resolver config should be valid");

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
        let mut metadata = Metadata::new(0, MessageType::Response, OpCode::Query);
        metadata.response_code = ResponseCode::ServFail;
        Header {
            metadata,
            counts: HeaderCounts::default(),
        }
        .into()
    }
}

#[async_trait::async_trait]
impl RequestHandler for BlockingDnsHandler {
    async fn handle_request<R: ResponseHandler, T: Time>(
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
                    let response = builder.error_msg(&request.metadata, ResponseCode::NXDomain);
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
                        request.metadata,
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
                let records: Vec<Record> = lookup.answers().to_vec();
                let response = builder.build(
                    request.metadata,
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
                let response = builder.error_msg(&request.metadata, ResponseCode::ServFail);
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
