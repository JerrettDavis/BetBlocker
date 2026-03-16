use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use scraper::{Html, Selector};
use url::Url;

use bb_common::enums::CrawlerSource;

use crate::discovery::crawler::{CrawlContext, CrawlError, CrawlResult, DomainCrawler};

/// Crawls gambling-affiliate directories and extracts outbound domains.
pub struct AffiliateCrawler {
    pub seed_urls: Vec<String>,
    pub max_depth: u32,
    pub link_selector: String,
}

impl AffiliateCrawler {
    #[must_use]
    pub fn new(seed_urls: Vec<String>, max_depth: u32, link_selector: String) -> Self {
        Self {
            seed_urls,
            max_depth,
            link_selector,
        }
    }

    /// Fetch a single URL and extract outbound domains.
    async fn fetch_and_extract(
        &self,
        ctx: &CrawlContext,
        url: &str,
    ) -> Result<Vec<CrawlResult>, CrawlError> {
        ctx.rate_limiter.acquire().await;

        let response = ctx
            .http
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            tracing::warn!(url = %url, status = %response.status(), "non-200 response, skipping");
            return Ok(Vec::new());
        }

        let body = response.text().await?;
        Self::extract_domains(&body, url, &self.link_selector)
    }

    /// Parse HTML and pull outbound domains from links matching the selector.
    fn extract_domains(
        html: &str,
        source_url: &str,
        selector_str: &str,
    ) -> Result<Vec<CrawlResult>, CrawlError> {
        let document = Html::parse_document(html);
        let selector = Selector::parse(selector_str)
            .map_err(|e| CrawlError::Parse(format!("bad CSS selector: {e:?}")))?;

        let source_domain = Url::parse(source_url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_default();

        let mut seen = HashSet::new();
        let mut results = Vec::new();

        for element in document.select(&selector) {
            let Some(href) = element.value().attr("href") else {
                continue;
            };

            // Resolve relative URLs against the source.
            let abs = match Url::parse(href) {
                Ok(u) => u,
                Err(_) => {
                    let Ok(base) = Url::parse(source_url) else {
                        continue;
                    };
                    match base.join(href) {
                        Ok(u) => u,
                        Err(_) => continue,
                    }
                }
            };

            // Only HTTP(S) links.
            if abs.scheme() != "http" && abs.scheme() != "https" {
                continue;
            }

            let Some(host) = abs.host_str() else {
                continue;
            };

            let domain = host.to_lowercase();

            // Skip self-links and duplicates.
            if domain == source_domain || !seen.insert(domain.clone()) {
                continue;
            }

            results.push(CrawlResult {
                domain,
                source_metadata: serde_json::json!({
                    "found_on": source_url,
                    "href": href,
                }),
            });
        }

        Ok(results)
    }
}

#[async_trait]
impl DomainCrawler for AffiliateCrawler {
    fn name(&self) -> &str {
        "affiliate"
    }

    fn source(&self) -> CrawlerSource {
        CrawlerSource::Affiliate
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError> {
        let mut all_results = Vec::new();
        let mut seen_domains: HashSet<String> = HashSet::new();

        // Depth-0: process seed URLs only. Deeper crawling is left for
        // future iterations (honour max_depth when implemented).
        let _ = self.max_depth; // acknowledge the field

        for seed in &self.seed_urls {
            match self.fetch_and_extract(ctx, seed).await {
                Ok(results) => {
                    for r in results {
                        if seen_domains.insert(r.domain.clone()) {
                            all_results.push(r);
                        }
                    }
                }
                Err(CrawlError::Http(e)) => {
                    tracing::warn!(url = %seed, error = %e, "HTTP error fetching seed URL, skipping");
                }
                Err(e) => {
                    tracing::warn!(url = %seed, error = %e, "error processing seed URL, skipping");
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

    const SAMPLE_HTML: &str = r#"
    <html>
    <body>
        <a href="https://casino-one.com/play">Casino One</a>
        <a href="https://sportsbet.example.org/signup?ref=aff123">SportsBet</a>
        <a href="https://internal.example.com/about">About</a>
        <a href="/relative-link">Relative</a>
        <a href="mailto:info@example.com">Email</a>
        <a href="https://casino-one.com/other-page">Casino One Again</a>
        <div class="ad">
            <a href="https://slots-world.net">Slots World</a>
        </div>
    </body>
    </html>
    "#;

    #[test]
    fn extracts_outbound_domains() {
        let results = AffiliateCrawler::extract_domains(
            SAMPLE_HTML,
            "https://internal.example.com/affiliates",
            "a",
        )
        .expect("should parse");

        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();

        assert!(
            domains.contains(&"casino-one.com"),
            "should find casino-one.com"
        );
        assert!(
            domains.contains(&"sportsbet.example.org"),
            "should find sportsbet.example.org"
        );
        assert!(
            domains.contains(&"slots-world.net"),
            "should find slots-world.net"
        );

        // Self-links to internal.example.com should be excluded.
        assert!(
            !domains.contains(&"internal.example.com"),
            "should not include source domain"
        );

        // Mailto links should be excluded.
        assert!(
            !domains.iter().any(|d| d.contains("mailto")),
            "should not include mailto"
        );
    }

    #[test]
    fn deduplicates_domains() {
        let results = AffiliateCrawler::extract_domains(
            SAMPLE_HTML,
            "https://internal.example.com/affiliates",
            "a",
        )
        .expect("should parse");

        let casino_count = results
            .iter()
            .filter(|r| r.domain == "casino-one.com")
            .count();

        assert_eq!(casino_count, 1, "casino-one.com should appear exactly once");
    }

    #[test]
    fn specific_selector_filters_elements() {
        let results = AffiliateCrawler::extract_domains(
            SAMPLE_HTML,
            "https://internal.example.com/affiliates",
            "div.ad a",
        )
        .expect("should parse");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].domain, "slots-world.net");
    }

    #[test]
    fn bad_selector_returns_error() {
        let result =
            AffiliateCrawler::extract_domains(SAMPLE_HTML, "https://example.com", "a[[[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn empty_html_returns_empty() {
        let results = AffiliateCrawler::extract_domains("", "https://example.com", "a")
            .expect("should parse");
        assert!(results.is_empty());
    }

    #[test]
    fn metadata_contains_source_info() {
        let results = AffiliateCrawler::extract_domains(
            SAMPLE_HTML,
            "https://internal.example.com/affiliates",
            "a",
        )
        .expect("should parse");

        let casino = results
            .iter()
            .find(|r| r.domain == "casino-one.com")
            .expect("should find casino-one.com");

        assert_eq!(
            casino.source_metadata["found_on"],
            "https://internal.example.com/affiliates"
        );
    }

    #[test]
    fn crawler_trait_name_and_source() {
        let crawler = AffiliateCrawler::new(vec![], 1, "a".to_string());
        assert_eq!(crawler.name(), "affiliate");
        assert_eq!(crawler.source(), CrawlerSource::Affiliate);
    }
}
