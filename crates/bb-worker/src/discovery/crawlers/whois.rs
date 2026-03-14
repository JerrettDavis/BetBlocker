use std::time::Duration;

use async_trait::async_trait;
use url::Url;

use bb_common::enums::CrawlerSource;

use crate::discovery::crawler::{CrawlContext, CrawlError, CrawlResult, DomainCrawler};

// ---------------------------------------------------------------------------
// Config / response types
// ---------------------------------------------------------------------------

/// A single result item returned by the WHOIS API.
#[derive(Debug, serde::Deserialize)]
pub struct WhoisRecord {
    /// The registered domain name.
    pub domain: String,
    /// Registrant organisation name, if available.
    #[serde(default)]
    pub registrant_org: Option<String>,
    /// Registrant e-mail address, if available.
    #[serde(default)]
    pub registrant_email: Option<String>,
}

/// The envelope returned by the WHOIS API.
///
/// We accept both `{"results": [...]}` and a bare `[...]`.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum WhoisApiResponse {
    Envelope { results: Vec<WhoisRecord> },
    Bare(Vec<WhoisRecord>),
}

impl WhoisApiResponse {
    pub fn into_records(self) -> Vec<WhoisRecord> {
        match self {
            Self::Envelope { results } => results,
            Self::Bare(v) => v,
        }
    }
}

// ---------------------------------------------------------------------------
// Crawler
// ---------------------------------------------------------------------------

/// Queries a WHOIS API service for recently registered domains whose
/// registrant details match known gambling-operator patterns.
pub struct WhoisCrawler {
    /// E-mail patterns / organisation names of known gambling operators.
    pub known_registrants: Vec<String>,
    /// TLDs to filter results to (e.g. `["com", "bet", "casino"]`).
    pub tlds: Vec<String>,
    /// Base URL of the WHOIS API endpoint.
    /// Expected to accept `?registrant=<pattern>` query parameters.
    pub api_endpoint: String,
}

impl WhoisCrawler {
    #[must_use]
    pub fn new(
        known_registrants: Vec<String>,
        tlds: Vec<String>,
        api_endpoint: impl Into<String>,
    ) -> Self {
        Self {
            known_registrants,
            tlds,
            api_endpoint: api_endpoint.into(),
        }
    }

    /// Default constructor with a reasonable set of known registrant patterns.
    #[must_use]
    pub fn with_defaults(api_endpoint: impl Into<String>) -> Self {
        Self::new(
            vec![
                "@gambling-provider.example".to_string(),
                "888holdings".to_string(),
                "betsson".to_string(),
                "kindred group".to_string(),
                "flutter entertainment".to_string(),
                "entain".to_string(),
            ],
            vec![
                "com".to_string(),
                "net".to_string(),
                "bet".to_string(),
                "casino".to_string(),
                "poker".to_string(),
                "bingo".to_string(),
            ],
            api_endpoint,
        )
    }

    /// Query the WHOIS API for a single registrant pattern and return raw
    /// records.
    async fn query_registrant(
        ctx: &CrawlContext,
        endpoint: &str,
        registrant: &str,
    ) -> Result<Vec<WhoisRecord>, CrawlError> {
        ctx.rate_limiter.acquire().await;

        let url = format!("{endpoint}?registrant={}", urlencoding_encode(registrant));

        let resp = ctx
            .http
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!(
                url = %url,
                status = %resp.status(),
                "non-200 from WHOIS API, skipping registrant"
            );
            return Ok(Vec::new());
        }

        let body = resp.text().await?;
        let api_response: WhoisApiResponse = serde_json::from_str(&body)
            .map_err(|e| CrawlError::Parse(format!("WHOIS API JSON parse error: {e}")))?;

        Ok(api_response.into_records())
    }

    /// Return `true` if the domain's TLD is in the configured TLD list.
    fn tld_matches(&self, domain: &str) -> bool {
        if self.tlds.is_empty() {
            return true;
        }
        let tld = domain.rsplit('.').next().unwrap_or("").to_lowercase();
        self.tlds.iter().any(|t| t.to_lowercase() == tld)
    }

    /// Parse a raw response body into `WhoisRecord`s without any HTTP call.
    /// Used by tests.
    pub fn parse_response(body: &str) -> Result<Vec<WhoisRecord>, CrawlError> {
        let api_response: WhoisApiResponse = serde_json::from_str(body)
            .map_err(|e| CrawlError::Parse(format!("WHOIS API JSON parse error: {e}")))?;
        Ok(api_response.into_records())
    }
}

/// Minimal percent-encoding for query parameter values.
///
/// Uses the `url` crate's `form_urlencoded` serialiser to produce the
/// encoded string, then extracts it from the raw query component so that
/// we get the percent-encoded bytes rather than the decoded form.
fn urlencoding_encode(s: &str) -> String {
    let mut url = Url::parse("https://placeholder.example/path").expect("valid base URL");
    url.query_pairs_mut().append_pair("v", s);
    // `url.query()` returns the raw percent-encoded query string "v=...".
    url.query()
        .and_then(|q| q.strip_prefix("v="))
        .map(str::to_string)
        .unwrap_or_else(|| s.to_string())
}

#[async_trait]
impl DomainCrawler for WhoisCrawler {
    fn name(&self) -> &str {
        "whois"
    }

    fn source(&self) -> CrawlerSource {
        CrawlerSource::WhoisPattern
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError> {
        let mut all_results = Vec::new();
        let mut seen_domains = std::collections::HashSet::new();

        for registrant_pattern in &self.known_registrants {
            let records = match Self::query_registrant(ctx, &self.api_endpoint, registrant_pattern)
                .await
            {
                Ok(r) => r,
                Err(CrawlError::Http(e)) => {
                    tracing::warn!(
                        pattern = %registrant_pattern,
                        error = %e,
                        "HTTP error querying WHOIS API, skipping pattern"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        pattern = %registrant_pattern,
                        error = %e,
                        "error querying WHOIS API, skipping pattern"
                    );
                    continue;
                }
            };

            for record in records {
                let domain = record.domain.to_lowercase();

                if !self.tld_matches(&domain) {
                    continue;
                }

                if seen_domains.insert(domain.clone()) {
                    all_results.push(CrawlResult {
                        domain,
                        source_metadata: serde_json::json!({
                            "registrant_pattern": registrant_pattern,
                            "registrant_org": record.registrant_org,
                            "registrant_email": record.registrant_email,
                        }),
                    });
                }
            }
        }

        Ok(all_results)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE_ENVELOPE: &str = r#"{
        "results": [
            {
                "domain": "new-casino.example.com",
                "registrant_org": "Betsson Group",
                "registrant_email": "domains@betsson.example"
            },
            {
                "domain": "poker-hub.example.bet",
                "registrant_org": "Betsson Group",
                "registrant_email": "domains@betsson.example"
            }
        ]
    }"#;

    const RESPONSE_BARE_ARRAY: &str = r#"[
        {
            "domain": "lucky-slots.example.casino",
            "registrant_org": "Kindred Group",
            "registrant_email": "reg@kindred.example"
        },
        {
            "domain": "minimal-domain.example.net"
        }
    ]"#;

    const RESPONSE_EMPTY: &str = r#"{"results": []}"#;

    // -----------------------------------------------------------------------
    // parse_response
    // -----------------------------------------------------------------------

    #[test]
    fn parse_envelope_response() {
        let records = WhoisCrawler::parse_response(RESPONSE_ENVELOPE).expect("should parse");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].domain, "new-casino.example.com");
        assert_eq!(
            records[0].registrant_org.as_deref(),
            Some("Betsson Group")
        );
        assert_eq!(records[1].domain, "poker-hub.example.bet");
    }

    #[test]
    fn parse_bare_array_response() {
        let records = WhoisCrawler::parse_response(RESPONSE_BARE_ARRAY).expect("should parse");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].domain, "lucky-slots.example.casino");
        assert!(records[1].registrant_org.is_none(), "optional org should be None");
    }

    #[test]
    fn parse_empty_results() {
        let records = WhoisCrawler::parse_response(RESPONSE_EMPTY).expect("should parse");
        assert!(records.is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = WhoisCrawler::parse_response("not json {{{");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // TLD filtering
    // -----------------------------------------------------------------------

    #[test]
    fn tld_filter_passes_matching_tld() {
        let crawler = WhoisCrawler::new(
            vec![],
            vec!["bet".to_string(), "casino".to_string()],
            "https://whois-api.example/",
        );
        assert!(crawler.tld_matches("example.bet"));
        assert!(crawler.tld_matches("slots.casino"));
        assert!(!crawler.tld_matches("boring.com"));
    }

    #[test]
    fn tld_filter_empty_list_accepts_all() {
        let crawler = WhoisCrawler::new(vec![], vec![], "https://whois-api.example/");
        assert!(crawler.tld_matches("anything.xyz"));
        assert!(crawler.tld_matches("example.com"));
    }

    // -----------------------------------------------------------------------
    // Crawler trait
    // -----------------------------------------------------------------------

    #[test]
    fn crawler_name_and_source() {
        let crawler = WhoisCrawler::new(vec![], vec![], "https://whois-api.example/");
        assert_eq!(crawler.name(), "whois");
        assert_eq!(crawler.source(), CrawlerSource::WhoisPattern);
    }

    #[test]
    fn with_defaults_has_registrants_and_tlds() {
        let crawler = WhoisCrawler::with_defaults("https://whois-api.example/");
        assert!(!crawler.known_registrants.is_empty());
        assert!(!crawler.tlds.is_empty());
        assert!(crawler
            .tlds
            .iter()
            .any(|t| t == "bet" || t == "casino" || t == "com"));
    }

    // -----------------------------------------------------------------------
    // urlencoding_encode helper
    // -----------------------------------------------------------------------

    #[test]
    fn url_encode_spaces_and_at() {
        let encoded = urlencoding_encode("kindred group");
        // Spaces should be encoded.
        assert!(!encoded.contains(' '));
    }

    #[test]
    fn url_encode_plain_ascii_unchanged() {
        let encoded = urlencoding_encode("betsson");
        assert_eq!(encoded, "betsson");
    }

    // -----------------------------------------------------------------------
    // crawl() with mock HTTP – no external calls
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn crawl_returns_empty_for_empty_registrants() {
        let crawler = WhoisCrawler::new(vec![], vec![], "https://whois-api.example/");
        let http = reqwest::Client::new();
        let ctx = crate::discovery::crawler::CrawlContext {
            http,
            rate_limiter: crate::discovery::crawler::RateLimiter::new(10, 10),
            last_run: None,
        };
        let results = crawler.crawl(&ctx).await.expect("should not error");
        assert!(results.is_empty());
    }
}
