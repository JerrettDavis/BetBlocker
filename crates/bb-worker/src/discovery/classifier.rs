use async_trait::async_trait;
use scraper::{Html, Selector};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Error type for classification failures.
#[derive(Debug, thiserror::Error)]
pub enum ClassifyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("parse error: {0}")]
    #[allow(dead_code)] // Valid error variant; will be constructed by future classifiers
    Parse(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Context passed to classifiers (shared HTTP client, etc.).
pub struct ClassifyContext {
    pub http: reqwest::Client,
}

/// Result of content classification for a domain.
#[derive(Debug, Clone)]
pub struct Classification {
    /// Score from keyword density analysis (0.0–1.0).
    pub keyword_score: f64,
    /// Score from HTML structure analysis (0.0–1.0).
    pub structure_score: f64,
    /// Score from link graph analysis (0.0–1.0).
    pub link_graph_score: f64,
    /// Best-guess gambling category, if identifiable.
    pub category_guess: Option<String>,
    /// Supporting evidence as structured JSON.
    pub evidence: serde_json::Value,
}

/// Trait for content classifiers.
#[async_trait]
pub trait ContentClassifier: Send + Sync {
    /// Classify a domain's content to determine gambling likelihood.
    async fn classify(
        &self,
        domain: &str,
        ctx: &ClassifyContext,
    ) -> Result<Classification, ClassifyError>;
}

// ---------------------------------------------------------------------------
// Rule-based classifier
// ---------------------------------------------------------------------------

/// Weighted gambling keyword used for density analysis.
struct WeightedKeyword {
    keyword: &'static str,
    weight: f64,
}

/// A rule-based content classifier that scores domains using keyword density,
/// HTML structure signals, and link graph analysis.
pub struct RuleBasedClassifier {
    keywords: Vec<WeightedKeyword>,
}

impl Default for RuleBasedClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleBasedClassifier {
    /// Create a new classifier with the default gambling keyword set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keywords: vec![
                WeightedKeyword {
                    keyword: "deposit bonus",
                    weight: 2.0,
                },
                WeightedKeyword {
                    keyword: "free spins",
                    weight: 2.0,
                },
                WeightedKeyword {
                    keyword: "casino",
                    weight: 1.5,
                },
                WeightedKeyword {
                    keyword: "poker",
                    weight: 1.5,
                },
                WeightedKeyword {
                    keyword: "slots",
                    weight: 1.5,
                },
                WeightedKeyword {
                    keyword: "bet",
                    weight: 1.0,
                },
                WeightedKeyword {
                    keyword: "wager",
                    weight: 1.0,
                },
                WeightedKeyword {
                    keyword: "odds",
                    weight: 1.0,
                },
                WeightedKeyword {
                    keyword: "18+",
                    weight: 1.0,
                },
            ],
        }
    }

    /// Compute keyword density score from page text.
    ///
    /// Returns a score in 0.0–1.0 based on weighted keyword matches
    /// normalised by word count.
    pub fn keyword_score(&self, text: &str) -> (f64, serde_json::Value) {
        let lower = text.to_lowercase();
        let word_count = lower.split_whitespace().count();

        if word_count == 0 {
            return (0.0, serde_json::json!({ "matches": [] }));
        }

        let mut total_weight = 0.0;
        let mut matches = Vec::new();

        for wk in &self.keywords {
            let count = lower.matches(wk.keyword).count();
            if count > 0 {
                total_weight += count as f64 * wk.weight;
                matches.push(serde_json::json!({
                    "keyword": wk.keyword,
                    "count": count,
                    "weight": wk.weight,
                }));
            }
        }

        // Normalise: weighted hits per 100 words, capped at 1.0.
        let density = (total_weight / word_count as f64) * 10.0;
        let score = density.min(1.0);

        (
            score,
            serde_json::json!({ "matches": matches, "word_count": word_count }),
        )
    }

    /// Analyse HTML structure for gambling-related elements.
    ///
    /// Looks for odds tables, deposit forms, and responsible-gambling links.
    /// Returns a score in 0.0–1.0.
    pub fn structure_score(&self, html: &str) -> (f64, serde_json::Value) {
        let document = Html::parse_document(html);
        let mut signals: Vec<String> = Vec::new();

        // Check for odds-style tables (tables with cells containing "+", "-", "/" patterns).
        if let Ok(table_sel) = Selector::parse("table") {
            for table in document.select(&table_sel) {
                let table_text = table.text().collect::<String>().to_lowercase();
                if table_text.contains("odds")
                    || table_text.contains("spread")
                    || table_text.contains("over/under")
                    || table_text.contains("moneyline")
                {
                    signals.push("odds_table".to_string());
                    break;
                }
            }
        }

        // Check for deposit forms (forms with inputs mentioning deposit/payment).
        if let Ok(form_sel) = Selector::parse("form") {
            for form in document.select(&form_sel) {
                let form_html = form.html().to_lowercase();
                if form_html.contains("deposit")
                    || form_html.contains("payment")
                    || form_html.contains("credit card")
                    || form_html.contains("withdraw")
                {
                    signals.push("deposit_form".to_string());
                    break;
                }
            }
        }

        // Check for responsible gambling links.
        if let Ok(link_sel) = Selector::parse("a") {
            for link in document.select(&link_sel) {
                let href = link.value().attr("href").unwrap_or("");
                let text = link.text().collect::<String>().to_lowercase();
                if text.contains("responsible gambling")
                    || text.contains("gamble responsibly")
                    || text.contains("gamble aware")
                    || href.contains("gambleaware")
                    || href.contains("begambleaware")
                    || href.contains("responsible-gambling")
                {
                    signals.push("responsible_gambling_link".to_string());
                    break;
                }
            }
        }

        // Each signal contributes equally; three signals = 1.0.
        let score = (signals.len() as f64 / 3.0).min(1.0);

        (score, serde_json::json!({ "signals": signals }))
    }

    /// Link graph score – stub returning 0.0.
    ///
    /// Future implementation will count outbound links to known gambling domains.
    pub fn link_graph_score(&self, _html: &str) -> (f64, serde_json::Value) {
        (
            0.0,
            serde_json::json!({ "note": "stub – not yet implemented" }),
        )
    }

    /// Guess the gambling category based on keyword evidence.
    pub fn guess_category(&self, text: &str) -> Option<String> {
        let lower = text.to_lowercase();

        // Order matters: more specific checks first.
        let categories: &[(&[&str], &str)] = &[
            (&["poker room", "poker tournament", "texas hold"], "poker"),
            (&["slot machine", "free spins", "slots"], "slots"),
            (
                &[
                    "sports betting",
                    "sportsbook",
                    "odds",
                    "moneyline",
                    "spread",
                ],
                "sports_betting",
            ),
            (
                &["casino", "blackjack", "roulette", "baccarat"],
                "online_casino",
            ),
            (&["lottery", "lotto", "draw"], "lottery"),
            (&["bingo"], "bingo"),
            (&["fantasy sports", "daily fantasy"], "fantasy_sports"),
            (
                &["crypto casino", "bitcoin gambling", "crypto betting"],
                "crypto_gambling",
            ),
            (&["affiliate", "partner program", "referral"], "affiliate"),
        ];

        for (keywords, category) in categories {
            if keywords.iter().any(|k| lower.contains(*k)) {
                return Some((*category).to_string());
            }
        }

        None
    }
}

#[async_trait]
impl ContentClassifier for RuleBasedClassifier {
    async fn classify(
        &self,
        domain: &str,
        ctx: &ClassifyContext,
    ) -> Result<Classification, ClassifyError> {
        let url = format!("https://{domain}");
        let response = ctx
            .http
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        let html = response.text().await?;
        let text = Html::parse_document(&html)
            .root_element()
            .text()
            .collect::<String>();

        let (keyword_score, kw_evidence) = self.keyword_score(&text);
        let (structure_score, struct_evidence) = self.structure_score(&html);
        let (link_graph_score, lg_evidence) = self.link_graph_score(&html);
        let category_guess = self.guess_category(&text);

        Ok(Classification {
            keyword_score,
            structure_score,
            link_graph_score,
            category_guess,
            evidence: serde_json::json!({
                "keyword": kw_evidence,
                "structure": struct_evidence,
                "link_graph": lg_evidence,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier() -> RuleBasedClassifier {
        RuleBasedClassifier::new()
    }

    // ── Keyword scoring ───────────────────────────────────────────────

    #[test]
    fn keyword_score_empty_text() {
        let (score, _) = classifier().keyword_score("");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn keyword_score_no_gambling_words() {
        let text = "Welcome to our news portal. We cover politics and technology.";
        let (score, _) = classifier().keyword_score(text);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn keyword_score_heavy_gambling() {
        let text = "casino casino casino bet bet odds odds poker slots \
                    deposit bonus free spins wager wager 18+ casino poker";
        let (score, evidence) = classifier().keyword_score(text);
        assert!(
            score > 0.5,
            "heavy gambling text should score high, got {score}"
        );
        let matches = evidence["matches"].as_array().unwrap();
        assert!(!matches.is_empty());
    }

    #[test]
    fn keyword_score_clamped_to_one() {
        // Extreme repetition should still cap at 1.0.
        let text = "casino ".repeat(500);
        let (score, _) = classifier().keyword_score(&text);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn keyword_score_case_insensitive() {
        let text = "CASINO Poker ODDS Bet";
        let (score, _) = classifier().keyword_score(text);
        assert!(score > 0.0, "should match case-insensitively");
    }

    // ── Structure scoring ─────────────────────────────────────────────

    #[test]
    fn structure_score_plain_html() {
        let html = "<html><body><p>Hello world</p></body></html>";
        let (score, _) = classifier().structure_score(html);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn structure_score_odds_table() {
        let html = r#"
        <html><body>
        <table><tr><td>Team A</td><td>Odds +150</td></tr></table>
        </body></html>
        "#;
        let (score, evidence) = classifier().structure_score(html);
        assert!(score > 0.0);
        let signals = evidence["signals"].as_array().unwrap();
        assert!(signals.iter().any(|s| s == "odds_table"));
    }

    #[test]
    fn structure_score_deposit_form() {
        let html = r#"
        <html><body>
        <form action="/pay"><input name="deposit" /><button>Deposit Now</button></form>
        </body></html>
        "#;
        let (score, evidence) = classifier().structure_score(html);
        assert!(score > 0.0);
        let signals = evidence["signals"].as_array().unwrap();
        assert!(signals.iter().any(|s| s == "deposit_form"));
    }

    #[test]
    fn structure_score_responsible_gambling_link() {
        let html = r#"
        <html><body>
        <a href="https://www.begambleaware.org">Gamble Responsibly</a>
        </body></html>
        "#;
        let (score, evidence) = classifier().structure_score(html);
        assert!(score > 0.0);
        let signals = evidence["signals"].as_array().unwrap();
        assert!(signals.iter().any(|s| s == "responsible_gambling_link"));
    }

    #[test]
    fn structure_score_all_signals() {
        let html = r#"
        <html><body>
        <table><tr><td>Odds +150</td></tr></table>
        <form><input name="deposit" /></form>
        <a href="https://www.begambleaware.org">Responsible Gambling</a>
        </body></html>
        "#;
        let (score, evidence) = classifier().structure_score(html);
        assert!(
            (score - 1.0).abs() < f64::EPSILON,
            "all 3 signals should give 1.0"
        );
        assert_eq!(evidence["signals"].as_array().unwrap().len(), 3);
    }

    // ── Link graph (stub) ─────────────────────────────────────────────

    #[test]
    fn link_graph_score_is_stub() {
        let (score, _) = classifier().link_graph_score("<html></html>");
        assert_eq!(score, 0.0);
    }

    // ── Category guessing ─────────────────────────────────────────────

    #[test]
    fn guess_category_casino() {
        let cat =
            classifier().guess_category("Welcome to our online casino with blackjack and roulette");
        assert_eq!(cat, Some("online_casino".to_string()));
    }

    #[test]
    fn guess_category_sports() {
        let cat = classifier().guess_category("Best sportsbook odds and moneyline bets");
        assert_eq!(cat, Some("sports_betting".to_string()));
    }

    #[test]
    fn guess_category_poker() {
        let cat = classifier().guess_category("Join our poker room for Texas Hold tournaments");
        assert_eq!(cat, Some("poker".to_string()));
    }

    #[test]
    fn guess_category_none() {
        let cat = classifier().guess_category("Buy fresh vegetables at our farm store");
        assert_eq!(cat, None);
    }

    #[test]
    fn guess_category_slots() {
        let cat = classifier().guess_category("Play our slot machine games with free spins");
        assert_eq!(cat, Some("slots".to_string()));
    }
}
