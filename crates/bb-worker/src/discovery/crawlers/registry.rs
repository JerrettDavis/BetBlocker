use std::time::Duration;

use async_trait::async_trait;
use scraper::{Html, Selector};
use url::Url;

use bb_common::enums::CrawlerSource;

use crate::discovery::crawler::{CrawlContext, CrawlError, CrawlResult, DomainCrawler};

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

/// How to parse a license registry page.
#[derive(Debug, Clone)]
pub enum RegistryParser {
    /// Extract domains from rows of an HTML table.
    HtmlTable,
    /// Fetch a JSON array / object from an API endpoint.
    #[allow(dead_code)] // Valid parser option; used when a registry exposes a JSON API
    JsonApi,
    /// Parse a comma-separated values file.
    #[allow(dead_code)] // Valid parser option; used when a registry publishes a CSV download
    Csv,
}

/// Configuration for a single license-registry source.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub parser: RegistryParser,
}

impl RegistryConfig {
    #[must_use]
    pub fn new(name: impl Into<String>, url: impl Into<String>, parser: RegistryParser) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            parser,
        }
    }
}

// ---------------------------------------------------------------------------
// Crawler
// ---------------------------------------------------------------------------

/// Fetches official gambling licence registries and extracts licensed
/// operator domains.
pub struct RegistryCrawler {
    pub registries: Vec<RegistryConfig>,
}

impl RegistryCrawler {
    #[must_use]
    pub fn new(registries: Vec<RegistryConfig>) -> Self {
        Self { registries }
    }

    /// Default set of well-known registries.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(vec![
            RegistryConfig::new(
                "UKGC",
                "https://www.gamblingcommission.gov.uk/public-register/businesses/entries",
                RegistryParser::HtmlTable,
            ),
            RegistryConfig::new(
                "MGA",
                "https://www.mga.org.mt/licence-holders/",
                RegistryParser::HtmlTable,
            ),
            RegistryConfig::new(
                "Curacao",
                "https://www.curacao-egaming.com/licensed-companies",
                RegistryParser::HtmlTable,
            ),
        ])
    }

    /// Fetch the registry URL and return the raw response body.
    async fn fetch_body(ctx: &CrawlContext, url: &str) -> Result<String, CrawlError> {
        ctx.rate_limiter.acquire().await;
        let resp = ctx
            .http
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!(url = %url, status = %resp.status(), "non-200 response from registry");
            return Ok(String::new());
        }

        Ok(resp.text().await?)
    }

    // -----------------------------------------------------------------------
    // Parsers
    // -----------------------------------------------------------------------

    /// Parse an HTML table page and extract any cell that looks like a domain.
    pub fn parse_html_table(html: &str, source_url: &str) -> Vec<CrawlResult> {
        let document = Html::parse_document(html);

        // Try `<a href>` elements first – many registries link to licensee sites.
        let link_sel = Selector::parse("table a[href]").unwrap_or_else(|_| {
            // Fallback: any anchor if the table-scoped one fails to parse.
            Selector::parse("a[href]").expect("infallible selector")
        });

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for el in document.select(&link_sel) {
            let Some(href) = el.value().attr("href") else {
                continue;
            };
            let domain = Self::href_to_domain(href, source_url);
            let Some(domain) = domain else { continue };
            if seen.insert(domain.clone()) {
                results.push(CrawlResult {
                    domain,
                    source_metadata: serde_json::json!({
                        "registry_url": source_url,
                        "parser": "html_table",
                    }),
                });
            }
        }

        // If no links found, scan table cells for bare domain-like text.
        if results.is_empty() {
            let td_sel =
                Selector::parse("table td").expect("infallible selector");
            for el in document.select(&td_sel) {
                let text = el.text().collect::<String>();
                let text = text.trim();
                if looks_like_domain(text) && seen.insert(text.to_string()) {
                    results.push(CrawlResult {
                        domain: text.to_lowercase(),
                        source_metadata: serde_json::json!({
                            "registry_url": source_url,
                            "parser": "html_table",
                        }),
                    });
                }
            }
        }

        results
    }

    /// Parse a JSON API response.  Handles both arrays of strings and arrays
    /// of objects that contain a `domain`, `website`, or `url` field.
    pub fn parse_json_api(body: &str, source_url: &str) -> Result<Vec<CrawlResult>, CrawlError> {
        let value: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| CrawlError::Parse(format!("JSON parse error: {e}")))?;

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let items = match &value {
            serde_json::Value::Array(arr) => arr.as_slice(),
            serde_json::Value::Object(map) => {
                // Common API envelope: {"data": [...]}
                if let Some(serde_json::Value::Array(arr)) = map.get("data") {
                    arr.as_slice()
                } else {
                    return Ok(results);
                }
            }
            _ => return Ok(results),
        };

        for item in items {
            let raw = match item {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Object(obj) => {
                    let candidate = obj
                        .get("domain")
                        .or_else(|| obj.get("website"))
                        .or_else(|| obj.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    candidate
                }
                _ => continue,
            };

            let domain = Self::href_to_domain(&raw, source_url)
                .or_else(|| looks_like_domain(&raw).then(|| raw.to_lowercase()));

            if let Some(domain) = domain {
                if seen.insert(domain.clone()) {
                    results.push(CrawlResult {
                        domain,
                        source_metadata: serde_json::json!({
                            "registry_url": source_url,
                            "parser": "json_api",
                        }),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Parse a CSV body where one column contains a domain or URL.
    /// Scans every field in every row.
    pub fn parse_csv(body: &str, source_url: &str) -> Vec<CrawlResult> {
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for line in body.lines() {
            for field in line.split(',') {
                let field = field.trim().trim_matches('"');
                let domain = Self::href_to_domain(field, source_url)
                    .or_else(|| looks_like_domain(field).then(|| field.to_lowercase()));
                if let Some(domain) = domain {
                    if seen.insert(domain.clone()) {
                        results.push(CrawlResult {
                            domain,
                            source_metadata: serde_json::json!({
                                "registry_url": source_url,
                                "parser": "csv",
                            }),
                        });
                    }
                }
            }
        }

        results
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Convert a raw href or URL string to a bare domain, or `None` if it
    /// cannot be parsed / resolved as an absolute or relative-path URL.
    ///
    /// Bare domain strings like `"casino.com"` intentionally return `None`
    /// here; callers should use `looks_like_domain` as a fallback for those.
    fn href_to_domain(raw: &str, source_url: &str) -> Option<String> {
        let parsed = if raw.starts_with("http://") || raw.starts_with("https://") {
            // Absolute URL – parse directly.
            Url::parse(raw).ok()?
        } else if raw.starts_with('/') || raw.starts_with("./") || raw.starts_with("../") {
            // Relative path – resolve against the source URL.
            let base = Url::parse(source_url).ok()?;
            base.join(raw).ok()?
        } else {
            // Bare domain or unknown format – signal to the caller to try the
            // `looks_like_domain` fallback by returning None.
            return None;
        };

        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return None;
        }

        Some(parsed.host_str()?.to_lowercase())
    }
}

#[async_trait]
impl DomainCrawler for RegistryCrawler {
    fn name(&self) -> &str {
        "registry"
    }

    fn source(&self) -> CrawlerSource {
        CrawlerSource::LicenseRegistry
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError> {
        let mut all_results = Vec::new();
        let mut seen_domains = std::collections::HashSet::new();

        for registry in &self.registries {
            let body = match Self::fetch_body(ctx, &registry.url).await {
                Ok(b) if b.is_empty() => continue,
                Ok(b) => b,
                Err(CrawlError::Http(e)) => {
                    tracing::warn!(
                        registry = %registry.name,
                        error = %e,
                        "HTTP error fetching registry, skipping"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        registry = %registry.name,
                        error = %e,
                        "error fetching registry, skipping"
                    );
                    continue;
                }
            };

            let results = match registry.parser {
                RegistryParser::HtmlTable => Self::parse_html_table(&body, &registry.url),
                RegistryParser::JsonApi => match Self::parse_json_api(&body, &registry.url) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(
                            registry = %registry.name,
                            error = %e,
                            "JSON parse error, skipping"
                        );
                        continue;
                    }
                },
                RegistryParser::Csv => Self::parse_csv(&body, &registry.url),
            };

            tracing::debug!(
                registry = %registry.name,
                count = results.len(),
                "parsed registry"
            );

            for r in results {
                if seen_domains.insert(r.domain.clone()) {
                    all_results.push(r);
                }
            }
        }

        Ok(all_results)
    }
}

// ---------------------------------------------------------------------------
// Domain-like heuristic
// ---------------------------------------------------------------------------

/// Very lightweight check: does this string look like a bare domain name?
/// E.g. "casino.com" → true, "some random text" → false.
fn looks_like_domain(s: &str) -> bool {
    if s.len() < 4 || s.len() > 253 {
        return false;
    }
    // Must contain a dot, no spaces, no slashes.
    if !s.contains('.') || s.contains(' ') || s.contains('/') {
        return false;
    }
    // Very rough TLD check: last segment must be 2-6 alpha chars.
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let tld = parts.last().expect("parts non-empty");
    tld.len() >= 2 && tld.len() <= 6 && tld.chars().all(|c| c.is_ascii_alphabetic())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const HTML_TABLE_FIXTURE: &str = r#"
    <html><body>
    <table>
        <tr><th>Operator</th><th>Website</th><th>Licence</th></tr>
        <tr>
            <td>Lucky Casino Ltd</td>
            <td><a href="https://luckycasino.example.com">luckycasino.example.com</a></td>
            <td>UK/12345</td>
        </tr>
        <tr>
            <td>Spin &amp; Win</td>
            <td><a href="https://spinwin.example.org/signup">spinwin.example.org</a></td>
            <td>UK/67890</td>
        </tr>
        <tr>
            <td>NoLink Betting</td>
            <td>nolink-betting.example.net</td>
            <td>UK/11111</td>
        </tr>
    </table>
    </body></html>
    "#;

    const HTML_TABLE_NO_LINKS_FIXTURE: &str = r#"
    <html><body>
    <table>
        <tr><th>Operator</th><th>Domain</th></tr>
        <tr><td>BetCo</td><td>betco.example.com</td></tr>
        <tr><td>WagerWorld</td><td>wagerworld.example.net</td></tr>
        <tr><td>Not a domain</td><td>just plain text</td></tr>
    </table>
    </body></html>
    "#;

    const JSON_ARRAY_FIXTURE: &str = r#"[
        "casino-alpha.example.com",
        "casino-beta.example.org",
        "notadomain"
    ]"#;

    const JSON_OBJECTS_FIXTURE: &str = r#"[
        {"name": "Alpha Casino", "domain": "alpha-casino.example.com"},
        {"name": "Beta Bets",   "website": "https://betabets.example.net/home"},
        {"name": "Gamma Games", "url": "http://gammagames.example.io"}
    ]"#;

    const JSON_ENVELOPE_FIXTURE: &str = r#"{
        "data": [
            {"domain": "envelope-casino.example.com"},
            {"domain": "envelope-bingo.example.org"}
        ]
    }"#;

    const CSV_FIXTURE: &str = r#"name,url,licence
"Ace Bets","https://acebets.example.com","MT/12345"
"Bingo Palace","bingo-palace.example.net","MT/67890"
"plain text","not-a-url-really","MT/99999"
"#;

    // -----------------------------------------------------------------------
    // HTML table parser
    // -----------------------------------------------------------------------

    #[test]
    fn html_table_extracts_linked_domains() {
        let results = RegistryCrawler::parse_html_table(
            HTML_TABLE_FIXTURE,
            "https://registry.example.gov/list",
        );
        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();

        assert!(domains.contains(&"luckycasino.example.com"), "should find luckycasino");
        assert!(domains.contains(&"spinwin.example.org"), "should find spinwin");
    }

    #[test]
    fn html_table_falls_back_to_cell_text() {
        let results = RegistryCrawler::parse_html_table(
            HTML_TABLE_NO_LINKS_FIXTURE,
            "https://registry.example.gov/list",
        );
        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();
        assert!(domains.contains(&"betco.example.com"), "should find betco.example.com");
        assert!(domains.contains(&"wagerworld.example.net"), "should find wagerworld");
        // Non-domain text cells should be ignored.
        assert!(!domains.contains(&"just plain text"), "should ignore non-domain text");
    }

    #[test]
    fn html_table_metadata_has_registry_url() {
        let results = RegistryCrawler::parse_html_table(
            HTML_TABLE_FIXTURE,
            "https://registry.example.gov/list",
        );
        assert!(!results.is_empty());
        assert_eq!(
            results[0].source_metadata["registry_url"],
            "https://registry.example.gov/list"
        );
        assert_eq!(results[0].source_metadata["parser"], "html_table");
    }

    #[test]
    fn html_table_empty_returns_empty() {
        let results = RegistryCrawler::parse_html_table("", "https://example.gov/list");
        assert!(results.is_empty());
    }

    // -----------------------------------------------------------------------
    // JSON API parser
    // -----------------------------------------------------------------------

    #[test]
    fn json_api_parses_string_array() {
        let results = RegistryCrawler::parse_json_api(
            JSON_ARRAY_FIXTURE,
            "https://api.example.gov/licensees",
        )
        .expect("should parse");
        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();
        assert!(domains.contains(&"casino-alpha.example.com"));
        assert!(domains.contains(&"casino-beta.example.org"));
        // "notadomain" should not appear.
        assert!(!domains.contains(&"notadomain"));
    }

    #[test]
    fn json_api_parses_object_array_domain_field() {
        let results = RegistryCrawler::parse_json_api(
            JSON_OBJECTS_FIXTURE,
            "https://api.example.gov/licensees",
        )
        .expect("should parse");
        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();
        assert!(domains.contains(&"alpha-casino.example.com"));
        assert!(domains.contains(&"betabets.example.net"));
        assert!(domains.contains(&"gammagames.example.io"));
    }

    #[test]
    fn json_api_parses_envelope_with_data_key() {
        let results = RegistryCrawler::parse_json_api(
            JSON_ENVELOPE_FIXTURE,
            "https://api.example.gov/licensees",
        )
        .expect("should parse");
        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();
        assert!(domains.contains(&"envelope-casino.example.com"));
        assert!(domains.contains(&"envelope-bingo.example.org"));
    }

    #[test]
    fn json_api_invalid_json_returns_error() {
        let result = RegistryCrawler::parse_json_api("not json {{{", "https://api.example.gov/");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // CSV parser
    // -----------------------------------------------------------------------

    #[test]
    fn csv_extracts_domains_and_urls() {
        let results = RegistryCrawler::parse_csv(CSV_FIXTURE, "https://registry.example.gov/csv");
        let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();
        assert!(domains.contains(&"acebets.example.com"), "should find acebets");
        assert!(domains.contains(&"bingo-palace.example.net"), "should find bingo-palace");
    }

    #[test]
    fn csv_empty_returns_empty() {
        let results = RegistryCrawler::parse_csv("", "https://registry.example.gov/csv");
        assert!(results.is_empty());
    }

    // -----------------------------------------------------------------------
    // Crawler trait
    // -----------------------------------------------------------------------

    #[test]
    fn crawler_name_and_source() {
        let crawler = RegistryCrawler::new(vec![]);
        assert_eq!(crawler.name(), "registry");
        assert_eq!(crawler.source(), CrawlerSource::LicenseRegistry);
    }

    #[test]
    fn with_defaults_has_registries() {
        let crawler = RegistryCrawler::with_defaults();
        assert!(!crawler.registries.is_empty());
        assert!(crawler.registries.iter().any(|r| r.name == "UKGC"));
        assert!(crawler.registries.iter().any(|r| r.name == "MGA"));
        assert!(crawler.registries.iter().any(|r| r.name == "Curacao"));
    }

    // -----------------------------------------------------------------------
    // looks_like_domain helper
    // -----------------------------------------------------------------------

    #[test]
    fn domain_heuristic() {
        assert!(looks_like_domain("example.com"));
        assert!(looks_like_domain("casino.co.uk"));
        assert!(!looks_like_domain("just text"));
        assert!(!looks_like_domain("no-dot"));
        assert!(!looks_like_domain("has/slash.com"));
        assert!(!looks_like_domain(""));
    }
}
