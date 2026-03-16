//! Token anonymization primitives for federated reporting.
//!
//! [`TokenRotator`] produces a daily-rotating pseudonym token via HMAC-SHA256
//! so that reports cannot be linked across days, while remaining consistent
//! within a single day.
//!
//! [`TemporalBucketer`] rounds timestamps down to the nearest hour so that
//! exact access times are not leaked in federated reports.

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generates a daily-rotating pseudonym token.
///
/// The token is `hex(HMAC-SHA256(seed, "YYYY-MM-DD"))`.  The same seed on the
/// same UTC calendar day always produces the same token; a different day
/// produces a different, unlinkable token.
#[derive(Clone)]
pub struct TokenRotator {
    seed: Vec<u8>,
}

impl TokenRotator {
    /// Create a rotator with a given seed (typically 32 random bytes that are
    /// persisted in the agent's local configuration so the token stays stable
    /// across process restarts within the same day).
    pub fn new(seed: Vec<u8>) -> Self {
        Self { seed }
    }

    /// Generate a fresh seed of `len` random bytes using the OS CSPRNG.
    pub fn generate_seed(len: usize) -> Vec<u8> {
        // Use ring's SystemRandom — already a dependency of bb-agent-core.
        use ring::rand::{SecureRandom, SystemRandom};
        let rng = SystemRandom::new();
        let mut buf = vec![0u8; len];
        rng.fill(&mut buf).expect("OS CSPRNG failed");
        buf
    }

    /// Return today's pseudonym token (UTC date).
    pub fn current_token(&self) -> String {
        self.token_for_date(Utc::now())
    }

    /// Return the pseudonym token for the UTC date of `dt`.
    pub fn token_for_date(&self, dt: DateTime<Utc>) -> String {
        let date_str = format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day());
        self.hmac_token(&date_str)
    }

    fn hmac_token(&self, message: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.seed).expect("HMAC accepts any key length");
        mac.update(message.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

/// Rounds timestamps down to the nearest UTC hour boundary.
///
/// Used to prevent exact access times from appearing in federated reports.
pub struct TemporalBucketer;

impl TemporalBucketer {
    /// Round `timestamp` down to the start of its UTC hour.
    ///
    /// # Examples
    ///
    /// `2024-06-15 14:37:22 UTC` → `2024-06-15 14:00:00 UTC`
    pub fn bucket(timestamp: DateTime<Utc>) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(
            timestamp.year(),
            timestamp.month(),
            timestamp.day(),
            timestamp.hour(),
            0,
            0,
        )
        .single()
        .expect("bucketed date always valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ── TokenRotator ────────────────────────────────────────────────────────

    #[test]
    fn same_seed_same_day_same_token() {
        let seed = b"test-seed-fixed-32-bytes-padding".to_vec();
        let rotator = TokenRotator::new(seed);
        let day = Utc.with_ymd_and_hms(2024, 6, 15, 14, 37, 0).unwrap();
        let token1 = rotator.token_for_date(day);
        let token2 = rotator.token_for_date(day);
        assert_eq!(
            token1, token2,
            "same seed + same day must produce same token"
        );
    }

    #[test]
    fn same_seed_different_day_different_token() {
        let seed = b"test-seed-fixed-32-bytes-padding".to_vec();
        let rotator = TokenRotator::new(seed);
        let day1 = Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        let day2 = Utc.with_ymd_and_hms(2024, 6, 16, 0, 0, 0).unwrap();
        let token1 = rotator.token_for_date(day1);
        let token2 = rotator.token_for_date(day2);
        assert_ne!(
            token1, token2,
            "different days must produce different tokens"
        );
    }

    #[test]
    fn different_seeds_same_day_different_token() {
        let rotator_a = TokenRotator::new(b"seed-aaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec());
        let rotator_b = TokenRotator::new(b"seed-bbbbbbbbbbbbbbbbbbbbbbbbbbb".to_vec());
        let day = Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        assert_ne!(
            rotator_a.token_for_date(day),
            rotator_b.token_for_date(day),
            "different seeds must produce different tokens even on the same day"
        );
    }

    #[test]
    fn token_is_hex_string() {
        let seed = TokenRotator::generate_seed(32);
        let rotator = TokenRotator::new(seed);
        let day = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let token = rotator.token_for_date(day);
        // HMAC-SHA256 output is 32 bytes = 64 hex chars
        assert_eq!(token.len(), 64, "token should be 64 hex characters");
        assert!(
            token.chars().all(|c| c.is_ascii_hexdigit()),
            "token should be all hex digits"
        );
    }

    // ── TemporalBucketer ────────────────────────────────────────────────────

    #[test]
    fn bucket_rounds_down_to_hour() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 14, 37, 22).unwrap();
        let bucketed = TemporalBucketer::bucket(ts);
        assert_eq!(
            bucketed,
            Utc.with_ymd_and_hms(2024, 6, 15, 14, 0, 0).unwrap()
        );
    }

    #[test]
    fn bucket_already_on_hour_boundary_unchanged() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 14, 0, 0).unwrap();
        let bucketed = TemporalBucketer::bucket(ts);
        assert_eq!(bucketed, ts);
    }

    #[test]
    fn bucket_midnight() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 0, 59, 59).unwrap();
        let bucketed = TemporalBucketer::bucket(ts);
        assert_eq!(
            bucketed,
            Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn bucket_end_of_day() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 23, 59, 59).unwrap();
        let bucketed = TemporalBucketer::bucket(ts);
        assert_eq!(
            bucketed,
            Utc.with_ymd_and_hms(2024, 6, 15, 23, 0, 0).unwrap()
        );
    }
}
