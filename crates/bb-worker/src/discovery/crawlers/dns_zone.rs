use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use bb_common::enums::CrawlerSource;

use crate::discovery::crawler::{CrawlContext, CrawlError, CrawlResult, DomainCrawler};

// ---------------------------------------------------------------------------
// Config / response types
// ---------------------------------------------------------------------------

/// A single domain entry from a zone file or enumeration API response.
#[derive(Debug, serde::Deserialize)]
pub struct ZoneEntry {
    /// The full domain name (e.g. `new-site.bet`).
    pub domain: String,
    /// Registration or first-seen timestamp (ISO-8601), if available.
    #[serde(default)]
    pub registered_at: Option<String>,
}

/// The envelope returned by the zone / enumeration API.
///
/// Accepts `{"domains": [...]}`, `{"results": [...]}`, or a bare `[...]`.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ZoneApiResponse {
    WithDomains { domains: Vec<ZoneEntry> },
    WithResults { results: Vec<ZoneEntry> },
    Bare(Vec<ZoneEntry>),
}

impl ZoneApiResponse {
    pub fn into_entries(self) -> Vec<ZoneEntry> {
        match self {
            Self::WithDomains { domains } => domains,
            Self::WithResults { results } => results,
            Self::Bare(v) => v,
        }
    }
}

// ---------------------------------------------------------------------------
// Crawler
// ---------------------------------------------------------------------------

/// Monitors gambling-specific TLDs for newly registered domains.
///
/// Fetches zone data via a CZDS-compatible API or an enumeration service.
/// When `last_run` is set in `CrawlContext`, only domains registered after
/// that timestamp are returned.
pub struct DnsZoneCrawler {
    /// TLDs to monitor (without leading dot, e.g. `"bet"`).
    pub tlds: Vec<String>,
    /// Base URL of the zone / CZDS API.
    /// Expected to accept `?tld=<tld>[&since=<iso8601>]` query parameters.
    pub api_endpoint: String,
}

impl DnsZoneCrawler {
    #[must_use]
    pub fn new(tlds: Vec<String>, api_endpoint: impl Into<String>) -> Self {
        Self {
            tlds,
            api_endpoint: api_endpoint.into(),
        }
    }

    /// Default constructor monitoring the five gambling-specific TLDs.
    #[must_use]
    pub fn with_defaults(api_endpoint: impl Into<String>) -> Self {
        Self::new(
            vec![
                "bet".to_string(),
                "casino".to_string(),
                "poker".to_string(),
                "games".to_string(),
                "bingo".to_string(),
            ],
            api_endpoint,
        )
    }

    /// Query the API for a single TLD since the given timestamp.
    async fn fetch_tld(
        ctx: &CrawlContext,
        endpoint: &str,
        tld: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ZoneEntry>, CrawlError> {
        ctx.rate_limiter.acquire().await;

        let mut url = format!("{endpoint}?tld={tld}");
        if let Some(ts) = since {
            url.push_str(&format!("&since={}", ts.to_rfc3339()));
        }

        let resp = ctx
            .http
            .get(&url)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!(
                tld = %tld,
                url = %url,
                status = %resp.status(),
                "non-200 from zone API, skipping TLD"
            );
            return Ok(Vec::new());
        }

        let body = resp.text().await?;
        Self::parse_response(&body)
    }

    /// Parse a raw zone API response body.
    /// Public so tests can call it without needing HTTP.
    pub fn parse_response(body: &str) -> Result<Vec<ZoneEntry>, CrawlError> {
        // Try JSON envelope formats first.
        if let Ok(api_resp) = serde_json::from_str::<ZoneApiResponse>(body) {
            return Ok(api_resp.into_entries());
        }

        // Fall back to plain-text zone file: one domain per line.
        Ok(Self::parse_zone_text(body))
    }

    /// Parse a plain-text zone file where each non-comment line that contains
    /// a dot is treated as a domain name.
    pub fn parse_zone_text(text: &str) -> Vec<ZoneEntry> {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with(';') && !line.starts_with('#'))
            .filter(|line| line.contains('.'))
            // Zone files sometimes have extra columns; take the first token.
            .filter_map(|line| {
                let domain = line
                    .split_whitespace()
                    .next()
                    .unwrap_or(line)
                    .trim_end_matches('.')
                    .to_lowercase();
                if domain.is_empty() { None } else { Some(ZoneEntry { domain, registered_at: None }) }
            })
            .collect()
    }

    /// Return `true` if the entry passes the `since` filter.
    fn passes_since_filter(entry: &ZoneEntry, since: Option<DateTime<Utc>>) -> bool {
        let Some(since) = since else {
            return true; // no filter → accept all
        };
        let Some(ts_str) = &entry.registered_at else {
            return true; // no timestamp on entry → accept (conservative)
        };
        // Parse as RFC-3339 / ISO-8601; if it fails, accept the entry.
        ts_str
            .parse::<DateTime<Utc>>()
            .map(|ts| ts > since)
            .unwrap_or(true)
    }
}

#[async_trait]
impl DomainCrawler for DnsZoneCrawler {
    fn name(&self) -> &str {
        "dns_zone"
    }

    fn source(&self) -> CrawlerSource {
        CrawlerSource::DnsZone
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError> {
        let since = ctx.last_run;
        let mut all_results = Vec::new();
        let mut seen_domains = std::collections::HashSet::new();

        for tld in &self.tlds {
            let entries =
                match Self::fetch_tld(ctx, &self.api_endpoint, tld, since).await {
                    Ok(e) => e,
                    Err(CrawlError::Http(e)) => {
                        tracing::warn!(
                            tld = %tld,
                            error = %e,
                            "HTTP error fetching zone data, skipping TLD"
                        );
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            tld = %tld,
                            error = %e,
                            "error fetching zone data, skipping TLD"
                        );
                        continue;
                    }
                };

            for entry in entries {
                if !Self::passes_since_filter(&entry, since) {
                    continue;
                }

                let domain = entry.domain.to_lowercase();
                if seen_domains.insert(domain.clone()) {
                    all_results.push(CrawlResult {
                        domain,
                        source_metadata: serde_json::json!({
                            "tld": tld,
                            "registered_at": entry.registered_at,
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

    const JSON_DOMAINS_FIXTURE: &str = r#"{
        "domains": [
            {"domain": "jackpot-city.bet", "registered_at": "2026-03-10T00:00:00Z"},
            {"domain": "royal-casino.casino", "registered_at": "2026-03-11T00:00:00Z"},
            {"domain": "no-timestamp.bet"}
        ]
    }"#;

    const JSON_RESULTS_FIXTURE: &str = r#"{
        "results": [
            {"domain": "holdem-poker.poker"},
            {"domain": "bingo-nights.bingo"}
        ]
    }"#;

    const JSON_BARE_ARRAY_FIXTURE: &str = r#"[
        {"domain": "arcade.games"},
        {"domain": "spin.bet"}
    ]"#;

    const ZONE_TEXT_FIXTURE: &str = r#"
; This is a zone file comment
# Also a comment
jackpot-city.bet.   IN  NS  ns1.example.com.
royal-casino.casino.   IN  NS  ns1.example.com.
just-domain.bet.

  ; blank lines and leading whitespace handled
"#;

    const ZONE_TEXT_PLAIN_FIXTURE: &str = "casino-alpha.bet\ncasino-beta.casino\n";

    // -----------------------------------------------------------------------
    // parse_response – JSON formats
    // -----------------------------------------------------------------------

    #[test]
    fn parse_json_with_domains_key() {
        let entries = DnsZoneCrawler::parse_response(JSON_DOMAINS_FIXTURE).expect("should parse");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].domain, "jackpot-city.bet");
        assert!(entries[0].registered_at.is_some());
        assert!(entries[2].registered_at.is_none());
    }

    #[test]
    fn parse_json_with_results_key() {
        let entries = DnsZoneCrawler::parse_response(JSON_RESULTS_FIXTURE).expect("should parse");
        assert_eq!(entries.len(), 2);
        let domains: Vec<&str> = entries.iter().map(|e| e.domain.as_str()).collect();
        assert!(domains.contains(&"holdem-poker.poker"));
        assert!(domains.contains(&"bingo-nights.bingo"));
    }

    #[test]
    fn parse_json_bare_array() {
        let entries =
            DnsZoneCrawler::parse_response(JSON_BARE_ARRAY_FIXTURE).expect("should parse");
        assert_eq!(entries.len(), 2);
    }

    // -----------------------------------------------------------------------
    // parse_zone_text – plain text zone files
    // -----------------------------------------------------------------------

    #[test]
    fn parse_zone_file_with_record_lines() {
        let entries = DnsZoneCrawler::parse_zone_text(ZONE_TEXT_FIXTURE);
        let domains: Vec<&str> = entries.iter().map(|e| e.domain.as_str()).collect();
        // Comments should be stripped.
        assert!(!domains.iter().any(|d| d.starts_with(';')));
        assert!(!domains.iter().any(|d| d.starts_with('#')));
        // Should extract from NS record lines (first token).
        assert!(domains.contains(&"jackpot-city.bet"), "should find jackpot-city.bet");
        assert!(domains.contains(&"royal-casino.casino"), "should find royal-casino.casino");
    }

    #[test]
    fn parse_zone_plain_one_per_line() {
        let entries = DnsZoneCrawler::parse_zone_text(ZONE_TEXT_PLAIN_FIXTURE);
        let domains: Vec<&str> = entries.iter().map(|e| e.domain.as_str()).collect();
        assert!(domains.contains(&"casino-alpha.bet"));
        assert!(domains.contains(&"casino-beta.casino"));
    }

    #[test]
    fn parse_zone_text_empty() {
        let entries = DnsZoneCrawler::parse_zone_text("");
        assert!(entries.is_empty());
    }

    // -----------------------------------------------------------------------
    // since filter
    // -----------------------------------------------------------------------

    #[test]
    fn since_filter_accepts_newer_entries() {
        let since: DateTime<Utc> = "2026-03-10T12:00:00Z".parse().unwrap();
        let entry = ZoneEntry {
            domain: "new.bet".to_string(),
            registered_at: Some("2026-03-11T00:00:00Z".to_string()),
        };
        assert!(DnsZoneCrawler::passes_since_filter(&entry, Some(since)));
    }

    #[test]
    fn since_filter_rejects_older_entries() {
        let since: DateTime<Utc> = "2026-03-10T12:00:00Z".parse().unwrap();
        let entry = ZoneEntry {
            domain: "old.bet".to_string(),
            registered_at: Some("2026-03-09T00:00:00Z".to_string()),
        };
        assert!(!DnsZoneCrawler::passes_since_filter(&entry, Some(since)));
    }

    #[test]
    fn since_filter_accepts_no_timestamp() {
        let since: DateTime<Utc> = "2026-03-10T12:00:00Z".parse().unwrap();
        let entry = ZoneEntry {
            domain: "unknown.bet".to_string(),
            registered_at: None,
        };
        assert!(DnsZoneCrawler::passes_since_filter(&entry, Some(since)));
    }

    #[test]
    fn since_filter_no_since_accepts_all() {
        let entry = ZoneEntry {
            domain: "any.casino".to_string(),
            registered_at: Some("2020-01-01T00:00:00Z".to_string()),
        };
        assert!(DnsZoneCrawler::passes_since_filter(&entry, None));
    }

    // -----------------------------------------------------------------------
    // Crawler trait
    // -----------------------------------------------------------------------

    #[test]
    fn crawler_name_and_source() {
        let crawler = DnsZoneCrawler::new(vec![], "https://czds-api.example/");
        assert_eq!(crawler.name(), "dns_zone");
        assert_eq!(crawler.source(), CrawlerSource::DnsZone);
    }

    #[test]
    fn with_defaults_has_gambling_tlds() {
        let crawler = DnsZoneCrawler::with_defaults("https://czds-api.example/");
        for tld in &["bet", "casino", "poker", "games", "bingo"] {
            assert!(
                crawler.tlds.iter().any(|t| t == tld),
                "missing TLD: {tld}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // crawl() with no TLDs → empty results
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn crawl_returns_empty_for_no_tlds() {
        let crawler = DnsZoneCrawler::new(vec![], "https://czds-api.example/");
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
