pub mod affiliate;
pub mod dns_zone;
pub mod registry;
pub mod search;
pub mod whois;

use crate::discovery::crawler::DomainCrawler;
use crate::discovery::crawlers::affiliate::AffiliateCrawler;
use crate::discovery::crawlers::dns_zone::DnsZoneCrawler;
use crate::discovery::crawlers::registry::{RegistryCrawler};
use crate::discovery::crawlers::search::SearchCrawler;
use crate::discovery::crawlers::whois::WhoisCrawler;

// ---------------------------------------------------------------------------
// Worker configuration
// ---------------------------------------------------------------------------

/// Runtime configuration for the worker process.
///
/// Each crawler section is `Option<…Config>` – when `None` the crawler is
/// disabled.  The inner config types are intentionally simple: callers
/// (tests, main, etc.) construct them directly.
#[derive(Debug, Default)]
pub struct WorkerConfig {
    /// Optional override of the WHOIS API base URL.
    pub whois_api_endpoint: Option<String>,
    /// Optional override of the DNS zone API base URL.
    pub dns_zone_api_endpoint: Option<String>,
    /// Optional SerpApi (or compatible) base URL.
    pub search_api_endpoint: Option<String>,
    /// Optional SerpApi key.
    pub search_api_key: Option<String>,
    /// Disable the affiliate crawler.
    pub affiliate_disabled: bool,
    /// Disable the registry crawler.
    pub registry_disabled: bool,
    /// Disable the WHOIS crawler.
    pub whois_disabled: bool,
    /// Disable the DNS zone crawler.
    pub dns_zone_disabled: bool,
    /// Disable the search crawler.
    pub search_disabled: bool,
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Instantiate all enabled crawlers from `config` and return them as a
/// heterogeneous `Vec<Box<dyn DomainCrawler>>`.
#[must_use]
pub fn build_crawlers(config: &WorkerConfig) -> Vec<Box<dyn DomainCrawler>> {
    let mut crawlers: Vec<Box<dyn DomainCrawler>> = Vec::new();

    // ── Affiliate ──────────────────────────────────────────────────────────
    if !config.affiliate_disabled {
        crawlers.push(Box::new(AffiliateCrawler::new(
            vec![
                "https://www.askgamblers.com/online-casinos/all".to_string(),
                "https://www.casinomeister.com/".to_string(),
            ],
            1,
            "a".to_string(),
        )));
    }

    // ── License Registry ───────────────────────────────────────────────────
    if !config.registry_disabled {
        crawlers.push(Box::new(RegistryCrawler::with_defaults()));
    }

    // ── WHOIS Pattern ──────────────────────────────────────────────────────
    if !config.whois_disabled {
        let endpoint = config
            .whois_api_endpoint
            .clone()
            .unwrap_or_else(|| "https://whois-api.example/v1/search".to_string());
        crawlers.push(Box::new(WhoisCrawler::with_defaults(endpoint)));
    }

    // ── DNS Zone ───────────────────────────────────────────────────────────
    if !config.dns_zone_disabled {
        let endpoint = config
            .dns_zone_api_endpoint
            .clone()
            .unwrap_or_else(|| "https://czds-api.example/v1/zones".to_string());
        crawlers.push(Box::new(DnsZoneCrawler::with_defaults(endpoint)));
    }

    // ── Search Engine ──────────────────────────────────────────────────────
    if !config.search_disabled {
        let endpoint = config
            .search_api_endpoint
            .clone()
            .unwrap_or_else(|| "https://serpapi.com/search".to_string());
        crawlers.push(Box::new(
            SearchCrawler::with_defaults(endpoint, config.search_api_key.clone())
                .with_delay_ms(10_000),
        ));
    }

    crawlers
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::crawlers::search::SearchEngine;

    // -----------------------------------------------------------------------
    // build_crawlers
    // -----------------------------------------------------------------------

    #[test]
    fn build_crawlers_default_config_returns_five_crawlers() {
        let config = WorkerConfig::default();
        let crawlers = build_crawlers(&config);
        assert_eq!(crawlers.len(), 5, "expected all 5 crawlers to be instantiated");
    }

    #[test]
    fn build_crawlers_disabled_flags_reduce_count() {
        let config = WorkerConfig {
            affiliate_disabled: true,
            registry_disabled: true,
            ..WorkerConfig::default()
        };
        let crawlers = build_crawlers(&config);
        assert_eq!(crawlers.len(), 3, "expected 3 enabled crawlers");
    }

    #[test]
    fn build_crawlers_all_disabled_returns_empty() {
        let config = WorkerConfig {
            affiliate_disabled: true,
            registry_disabled: true,
            whois_disabled: true,
            dns_zone_disabled: true,
            search_disabled: true,
            ..WorkerConfig::default()
        };
        let crawlers = build_crawlers(&config);
        assert!(crawlers.is_empty());
    }

    #[test]
    fn build_crawlers_names_are_correct() {
        let config = WorkerConfig::default();
        let crawlers = build_crawlers(&config);
        let names: Vec<&str> = crawlers.iter().map(|c| c.name()).collect();
        assert!(names.contains(&"affiliate"));
        assert!(names.contains(&"registry"));
        assert!(names.contains(&"whois"));
        assert!(names.contains(&"dns_zone"));
        assert!(names.contains(&"search"));
    }

    #[test]
    fn build_crawlers_custom_endpoints() {
        let config = WorkerConfig {
            whois_api_endpoint: Some("https://custom-whois.example/api".to_string()),
            dns_zone_api_endpoint: Some("https://custom-czds.example/api".to_string()),
            search_api_endpoint: Some("https://custom-search.example/api".to_string()),
            search_api_key: Some("sk-test-key".to_string()),
            ..WorkerConfig::default()
        };
        let crawlers = build_crawlers(&config);
        // Simply verify they all construct without panicking.
        assert_eq!(crawlers.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Integration: all crawlers return empty results for no-data mocks
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn all_crawlers_return_empty_with_no_registrants_or_tlds() {
        // Build minimal crawlers that won't make any real HTTP calls:
        // – empty seed URLs for affiliate
        // – empty registries for registry
        // – empty registrants for whois
        // – empty tlds for dns_zone
        // – empty queries for search

        use crate::discovery::crawler::{CrawlContext, RateLimiter};

        let crawlers: Vec<Box<dyn DomainCrawler>> = vec![
            Box::new(AffiliateCrawler::new(vec![], 1, "a".to_string())),
            Box::new(RegistryCrawler::new(vec![])),
            Box::new(WhoisCrawler::new(
                vec![],
                vec![],
                "https://whois-api.example/",
            )),
            Box::new(DnsZoneCrawler::new(vec![], "https://czds-api.example/")),
            Box::new(SearchCrawler::new(
                vec![],
                SearchEngine::Google,
                10,
                "https://serpapi.com/search",
                None,
            )),
        ];

        let http = reqwest::Client::new();
        let ctx = CrawlContext {
            http,
            rate_limiter: RateLimiter::new(10, 10),
            last_run: None,
        };

        for crawler in &crawlers {
            let results = crawler
                .crawl(&ctx)
                .await
                .expect("crawler should not return Err with empty config");
            assert!(
                results.is_empty(),
                "crawler '{}' should return empty results when configured with no sources",
                crawler.name()
            );
        }
    }
}
