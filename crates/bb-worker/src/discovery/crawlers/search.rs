use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use url::Url;

use bb_common::enums::CrawlerSource;

use crate::discovery::crawler::{CrawlContext, CrawlError, CrawlResult, DomainCrawler};

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

/// Supported search engine backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchEngine {
    /// Google via SerpApi (`https://serpapi.com/search`).
    Google,
    /// Bing via SerpApi or the Bing Web Search API.
    #[allow(dead_code)] // Valid engine option; available for callers to configure
    Bing,
    /// DuckDuckGo HTML search (no official API key required for low volumes).
    #[allow(dead_code)] // Valid engine option; available for callers to configure
    DuckDuckGo,
}

impl SearchEngine {
    /// Human-readable name used in metadata.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Google => "google",
            Self::Bing => "bing",
            Self::DuckDuckGo => "duckduckgo",
        }
    }
}

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// A single organic result as returned by SerpApi or compatible services.
#[derive(Debug, serde::Deserialize)]
pub struct SearchResult {
    /// The page URL.
    pub link: String,
    /// Optional display title (used only for debug logging).
    #[serde(default)]
    pub title: Option<String>,
}

/// SerpApi-style response envelope.
#[derive(Debug, serde::Deserialize)]
pub struct SearchApiResponse {
    /// Organic search results.
    #[serde(default, rename = "organic_results")]
    pub organic_results: Vec<SearchResult>,
}

// ---------------------------------------------------------------------------
// Crawler
// ---------------------------------------------------------------------------

/// Executes keyword queries against a search API and extracts result domains.
pub struct SearchCrawler {
    /// Gambling keyword queries to issue (one API call per query).
    pub queries: Vec<String>,
    /// Which search engine to use.
    pub engine: SearchEngine,
    /// Number of results to request per query.
    pub results_per_query: u32,
    /// Search API base URL.
    /// For SerpApi: `https://serpapi.com/search`
    pub api_endpoint: String,
    /// Optional API key (SerpApi `api_key` parameter).
    pub api_key: Option<String>,
    /// Delay between consecutive queries in milliseconds (default: 10 000 ms).
    pub inter_query_delay_ms: u64,
}

impl SearchCrawler {
    #[must_use]
    pub fn new(
        queries: Vec<String>,
        engine: SearchEngine,
        results_per_query: u32,
        api_endpoint: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            queries,
            engine,
            results_per_query,
            api_endpoint: api_endpoint.into(),
            api_key,
            inter_query_delay_ms: 10_000,
        }
    }

    /// Builder: override the inter-query delay (mainly for tests).
    #[must_use]
    pub fn with_delay_ms(mut self, ms: u64) -> Self {
        self.inter_query_delay_ms = ms;
        self
    }

    /// Default constructor with common gambling keyword queries.
    #[must_use]
    pub fn with_defaults(
        api_endpoint: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self::new(
            vec![
                "online casino real money".to_string(),
                "sports betting site".to_string(),
                "poker online play".to_string(),
                "online bingo site".to_string(),
                "best slots site".to_string(),
            ],
            SearchEngine::Google,
            10,
            api_endpoint,
            api_key,
        )
    }

    /// Build the full request URL for a single query.
    fn build_request_url(&self, query: &str) -> Result<String, CrawlError> {
        let mut url = Url::parse(&self.api_endpoint)
            .map_err(|e| CrawlError::Parse(format!("invalid API endpoint: {e}")))?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("q", query);
            pairs.append_pair("num", &self.results_per_query.to_string());
            pairs.append_pair("engine", self.engine.as_str());
            if let Some(key) = &self.api_key {
                pairs.append_pair("api_key", key);
            }
        }

        Ok(url.to_string())
    }

    /// Execute a single search query and return the parsed response.
    async fn execute_query(
        &self,
        ctx: &CrawlContext,
        query: &str,
    ) -> Result<SearchApiResponse, CrawlError> {
        ctx.rate_limiter.acquire().await;

        let url = self.build_request_url(query)?;

        let resp = ctx
            .http
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!(
                query = %query,
                url = %url,
                status = %resp.status(),
                "non-200 from search API"
            );
            return Ok(SearchApiResponse {
                organic_results: Vec::new(),
            });
        }

        let body = resp.text().await?;
        Self::parse_response(&body)
    }

    /// Parse a raw search API response body.  Public so tests can call it
    /// directly without HTTP.
    pub fn parse_response(body: &str) -> Result<SearchApiResponse, CrawlError> {
        serde_json::from_str(body)
            .map_err(|e| CrawlError::Parse(format!("search API JSON parse error: {e}")))
    }

    /// Extract the bare domain from a `link` field.
    pub fn extract_domain(link: &str) -> Option<String> {
        let url = Url::parse(link).ok()?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return None;
        }
        Some(url.host_str()?.to_lowercase())
    }
}

#[async_trait]
impl DomainCrawler for SearchCrawler {
    fn name(&self) -> &str {
        "search"
    }

    fn source(&self) -> CrawlerSource {
        CrawlerSource::SearchEngine
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError> {
        let mut all_results: Vec<CrawlResult> = Vec::new();
        let mut seen_domains: HashSet<String> = HashSet::new();

        for (idx, query) in self.queries.iter().enumerate() {
            // Aggressive rate-limiting: sleep between queries (except before
            // the very first one).
            if idx > 0 && self.inter_query_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.inter_query_delay_ms)).await;
            }

            let api_response = match self.execute_query(ctx, query).await {
                Ok(r) => r,
                Err(CrawlError::Http(e)) => {
                    tracing::warn!(
                        query = %query,
                        error = %e,
                        "HTTP error during search query, skipping"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        query = %query,
                        error = %e,
                        "error during search query, skipping"
                    );
                    continue;
                }
            };

            for result in api_response.organic_results {
                let Some(domain) = Self::extract_domain(&result.link) else {
                    continue;
                };

                if seen_domains.insert(domain.clone()) {
                    all_results.push(CrawlResult {
                        domain,
                        source_metadata: serde_json::json!({
                            "query": query,
                            "engine": self.engine.as_str(),
                            "link": result.link,
                            "title": result.title,
                        }),
                    });
                }
            }

            tracing::debug!(
                query = %query,
                "search query completed"
            );
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

    const SERPAPI_RESPONSE: &str = r#"{
        "organic_results": [
            {
                "link": "https://casino-alpha.example.com/play",
                "title": "Casino Alpha – Best Online Casino"
            },
            {
                "link": "https://betabets.example.net/signup",
                "title": "BetaBets – Sports Betting"
            },
            {
                "link": "https://casino-alpha.example.com/slots",
                "title": "Casino Alpha – Slots"
            }
        ]
    }"#;

    const EMPTY_RESPONSE: &str = r#"{"organic_results": []}"#;

    const MINIMAL_RESPONSE: &str = r#"{
        "organic_results": [
            {"link": "https://only-link.example.org"}
        ]
    }"#;

    // -----------------------------------------------------------------------
    // parse_response
    // -----------------------------------------------------------------------

    #[test]
    fn parse_serpapi_response() {
        let parsed = SearchCrawler::parse_response(SERPAPI_RESPONSE).expect("should parse");
        assert_eq!(parsed.organic_results.len(), 3);
        assert_eq!(parsed.organic_results[0].link, "https://casino-alpha.example.com/play");
        assert_eq!(
            parsed.organic_results[0].title.as_deref(),
            Some("Casino Alpha – Best Online Casino")
        );
    }

    #[test]
    fn parse_empty_results() {
        let parsed = SearchCrawler::parse_response(EMPTY_RESPONSE).expect("should parse");
        assert!(parsed.organic_results.is_empty());
    }

    #[test]
    fn parse_result_with_no_title() {
        let parsed = SearchCrawler::parse_response(MINIMAL_RESPONSE).expect("should parse");
        assert_eq!(parsed.organic_results.len(), 1);
        assert!(parsed.organic_results[0].title.is_none());
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = SearchCrawler::parse_response("not json {{{");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // extract_domain
    // -----------------------------------------------------------------------

    #[test]
    fn extract_domain_from_full_url() {
        assert_eq!(
            SearchCrawler::extract_domain("https://casino-alpha.example.com/play"),
            Some("casino-alpha.example.com".to_string())
        );
    }

    #[test]
    fn extract_domain_lowercases() {
        assert_eq!(
            SearchCrawler::extract_domain("https://CASINO.EXAMPLE.COM/"),
            Some("casino.example.com".to_string())
        );
    }

    #[test]
    fn extract_domain_non_http_returns_none() {
        assert_eq!(SearchCrawler::extract_domain("ftp://files.example.com"), None);
        assert_eq!(SearchCrawler::extract_domain("mailto:info@example.com"), None);
    }

    #[test]
    fn extract_domain_invalid_url_returns_none() {
        assert_eq!(SearchCrawler::extract_domain("not a url"), None);
    }

    // -----------------------------------------------------------------------
    // build_request_url
    // -----------------------------------------------------------------------

    #[test]
    fn build_request_url_includes_query_and_engine() {
        let crawler = SearchCrawler::new(
            vec![],
            SearchEngine::Google,
            10,
            "https://serpapi.com/search",
            Some("test-key".to_string()),
        );
        let url = crawler
            .build_request_url("online casino")
            .expect("should build URL");
        assert!(url.contains("engine=google"), "should include engine");
        assert!(url.contains("api_key=test-key"), "should include api_key");
        assert!(url.contains("num=10"), "should include num");
    }

    #[test]
    fn build_request_url_no_api_key() {
        let crawler = SearchCrawler::new(
            vec![],
            SearchEngine::DuckDuckGo,
            5,
            "https://search-api.example/",
            None,
        );
        let url = crawler
            .build_request_url("poker sites")
            .expect("should build URL");
        assert!(url.contains("engine=duckduckgo"));
        assert!(!url.contains("api_key"), "should not include api_key");
    }

    // -----------------------------------------------------------------------
    // deduplication
    // -----------------------------------------------------------------------

    #[test]
    fn deduplicate_domains_across_results() {
        // casino-alpha.example.com appears twice in SERPAPI_RESPONSE.
        let parsed = SearchCrawler::parse_response(SERPAPI_RESPONSE).expect("should parse");
        let mut seen = HashSet::new();
        let mut unique = Vec::new();
        for result in parsed.organic_results {
            if let Some(domain) = SearchCrawler::extract_domain(&result.link) {
                if seen.insert(domain.clone()) {
                    unique.push(domain);
                }
            }
        }
        let count = unique.iter().filter(|d| *d == "casino-alpha.example.com").count();
        assert_eq!(count, 1, "casino-alpha.example.com should appear exactly once");
    }

    // -----------------------------------------------------------------------
    // Crawler trait
    // -----------------------------------------------------------------------

    #[test]
    fn crawler_name_and_source() {
        let crawler = SearchCrawler::new(
            vec![],
            SearchEngine::Google,
            10,
            "https://serpapi.com/search",
            None,
        );
        assert_eq!(crawler.name(), "search");
        assert_eq!(crawler.source(), CrawlerSource::SearchEngine);
    }

    #[test]
    fn with_defaults_has_queries() {
        let crawler = SearchCrawler::with_defaults("https://serpapi.com/search", None);
        assert!(!crawler.queries.is_empty());
        assert_eq!(crawler.engine, SearchEngine::Google);
        assert_eq!(crawler.inter_query_delay_ms, 10_000);
    }

    #[test]
    fn search_engine_as_str() {
        assert_eq!(SearchEngine::Google.as_str(), "google");
        assert_eq!(SearchEngine::Bing.as_str(), "bing");
        assert_eq!(SearchEngine::DuckDuckGo.as_str(), "duckduckgo");
    }

    // -----------------------------------------------------------------------
    // crawl() with no queries → empty results
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn crawl_returns_empty_for_no_queries() {
        let crawler = SearchCrawler::new(
            vec![],
            SearchEngine::Google,
            10,
            "https://serpapi.com/search",
            None,
        );
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
