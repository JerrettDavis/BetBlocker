use super::classifier::Classification;

// ---------------------------------------------------------------------------
// Score category
// ---------------------------------------------------------------------------

/// Categorisation of a confidence score into an action tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreCategory {
    /// Score < 0.3 – unlikely to be gambling, discard.
    Discard,
    /// Score 0.3–0.85 – needs human review.
    StandardReview,
    /// Score > 0.85 – very likely gambling, fast-track.
    HighPriority,
}

// ---------------------------------------------------------------------------
// Confidence scorer
// ---------------------------------------------------------------------------

/// Computes a weighted confidence score from a [`Classification`].
#[derive(Debug, Clone)]
pub struct ConfidenceScorer {
    pub keyword_weight: f64,
    pub structure_weight: f64,
    pub link_graph_weight: f64,
}

impl Default for ConfidenceScorer {
    fn default() -> Self {
        Self {
            keyword_weight: 0.4,
            structure_weight: 0.3,
            link_graph_weight: 0.3,
        }
    }
}

impl ConfidenceScorer {
    /// Create a scorer with custom weights.
    #[must_use]
    pub fn new(keyword_weight: f64, structure_weight: f64, link_graph_weight: f64) -> Self {
        Self {
            keyword_weight,
            structure_weight,
            link_graph_weight,
        }
    }

    /// Compute the weighted confidence score, clamped to 0.0–1.0.
    #[must_use]
    pub fn score(&self, classification: &Classification) -> f64 {
        let raw = classification.keyword_score * self.keyword_weight
            + classification.structure_score * self.structure_weight
            + classification.link_graph_score * self.link_graph_weight;

        raw.clamp(0.0, 1.0)
    }

    /// Map a confidence score to a [`ScoreCategory`].
    #[must_use]
    pub fn categorize(&self, score: f64) -> ScoreCategory {
        if score > 0.85 {
            ScoreCategory::HighPriority
        } else if score >= 0.3 {
            ScoreCategory::StandardReview
        } else {
            ScoreCategory::Discard
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_classification(keyword: f64, structure: f64, link_graph: f64) -> Classification {
        Classification {
            keyword_score: keyword,
            structure_score: structure,
            link_graph_score: link_graph,
            category_guess: None,
            evidence: serde_json::json!({}),
        }
    }

    fn scorer() -> ConfidenceScorer {
        ConfidenceScorer::default()
    }

    // ── Weighted scoring ──────────────────────────────────────────────

    #[test]
    fn score_all_zeros() {
        let c = make_classification(0.0, 0.0, 0.0);
        assert_eq!(scorer().score(&c), 0.0);
    }

    #[test]
    fn score_all_ones() {
        let c = make_classification(1.0, 1.0, 1.0);
        let s = scorer().score(&c);
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_weighted_correctly() {
        // keyword=0.5 * 0.4 = 0.20
        // structure=0.8 * 0.3 = 0.24
        // link_graph=0.0 * 0.3 = 0.00
        // total = 0.44
        let c = make_classification(0.5, 0.8, 0.0);
        let s = scorer().score(&c);
        assert!((s - 0.44).abs() < 1e-10, "expected 0.44, got {s}");
    }

    #[test]
    fn score_clamped_to_max() {
        // Even if input scores exceed 1.0 somehow, output should be clamped.
        let c = make_classification(2.0, 2.0, 2.0);
        let s = scorer().score(&c);
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_clamped_to_min() {
        let c = make_classification(-1.0, -1.0, -1.0);
        let s = scorer().score(&c);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn score_custom_weights() {
        let custom = ConfidenceScorer::new(1.0, 0.0, 0.0);
        let c = make_classification(0.7, 1.0, 1.0);
        let s = custom.score(&c);
        assert!((s - 0.7).abs() < 1e-10, "only keyword weight should matter, got {s}");
    }

    // ── Threshold categorisation ──────────────────────────────────────

    #[test]
    fn categorize_discard_zero() {
        assert_eq!(scorer().categorize(0.0), ScoreCategory::Discard);
    }

    #[test]
    fn categorize_discard_below_threshold() {
        assert_eq!(scorer().categorize(0.29), ScoreCategory::Discard);
    }

    #[test]
    fn categorize_standard_review_at_lower_boundary() {
        assert_eq!(scorer().categorize(0.3), ScoreCategory::StandardReview);
    }

    #[test]
    fn categorize_standard_review_mid() {
        assert_eq!(scorer().categorize(0.5), ScoreCategory::StandardReview);
    }

    #[test]
    fn categorize_standard_review_at_upper_boundary() {
        assert_eq!(scorer().categorize(0.85), ScoreCategory::StandardReview);
    }

    #[test]
    fn categorize_high_priority_above_threshold() {
        assert_eq!(scorer().categorize(0.86), ScoreCategory::HighPriority);
    }

    #[test]
    fn categorize_high_priority_max() {
        assert_eq!(scorer().categorize(1.0), ScoreCategory::HighPriority);
    }

    // ── Integration: score → categorize ───────────────────────────────

    #[test]
    fn full_pipeline_high_gambling_signal() {
        let c = make_classification(1.0, 1.0, 1.0);
        let s = scorer();
        let score = s.score(&c);
        assert_eq!(s.categorize(score), ScoreCategory::HighPriority);
    }

    #[test]
    fn full_pipeline_low_signal() {
        let c = make_classification(0.1, 0.0, 0.0);
        let s = scorer();
        let score = s.score(&c);
        assert_eq!(s.categorize(score), ScoreCategory::Discard);
    }

    #[test]
    fn full_pipeline_moderate_signal() {
        let c = make_classification(0.6, 0.5, 0.0);
        let s = scorer();
        let score = s.score(&c);
        // 0.6*0.4 + 0.5*0.3 + 0.0*0.3 = 0.24 + 0.15 = 0.39
        assert_eq!(s.categorize(score), ScoreCategory::StandardReview);
    }
}
