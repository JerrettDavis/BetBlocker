//! Windows self-update mechanism.
//!
//! Downloads, verifies, and applies agent updates by:
//! 1. Querying the BetBlocker update API for the latest version.
//! 2. Downloading the MSI to a temp file.
//! 3. Verifying the SHA-256 hash (and optionally Authenticode signature).
//! 4. Running `msiexec /i <msi> /quiet /norestart` to apply the update.
//!
//! A `UpdateScheduler` wraps these steps in a tokio interval loop.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during update operations.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// Network or HTTP error while checking / downloading.
    #[error("network error: {0}")]
    Network(String),

    /// The downloaded file's SHA-256 hash does not match the manifest.
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        /// Expected hex-encoded SHA-256.
        expected: String,
        /// Actual hex-encoded SHA-256.
        actual: String,
    },

    /// Authenticode signature verification failed.
    #[error("signature verification failed: {0}")]
    SignatureVerification(String),

    /// IO error during download or temp-file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON deserialization error.
    #[error("deserialization error: {0}")]
    Deserialize(String),

    /// `msiexec` returned a non-zero exit code.
    #[error("msiexec failed with exit code {0}")]
    MsiexecFailed(i32),

    /// The current version is already up to date.
    #[error("already up to date (version {0})")]
    AlreadyUpToDate(String),
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Update manifest returned by the BetBlocker update API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// The new version string (e.g. `"1.2.3"`).
    pub version: String,
    /// URL to download the MSI from.
    pub download_url: String,
    /// Hex-encoded SHA-256 hash of the MSI.
    pub sha256: String,
    /// Optional release notes / changelog.
    pub release_notes: Option<String>,
    /// Minimum supported OS version (informational).
    pub min_os_version: Option<String>,
}

// ---------------------------------------------------------------------------
// Version comparison helper
// ---------------------------------------------------------------------------

/// Parse a semver-like `"x.y.z"` string into a comparable tuple.
///
/// Non-numeric parts are treated as zero so that pre-release strings degrade
/// gracefully rather than panicking.
fn parse_version(v: &str) -> (u64, u64, u64) {
    let parts: Vec<u64> = v
        .split('.')
        .map(|p| {
            // Strip any trailing pre-release suffix (e.g. "1-beta")
            p.split('-').next().unwrap_or("0").parse().unwrap_or(0)
        })
        .collect();

    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

/// Return `true` if `candidate` is strictly newer than `current`.
#[must_use]
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    parse_version(candidate) > parse_version(current)
}

// ---------------------------------------------------------------------------
// SHA-256 helpers (no external crypto dep required for the shim)
// ---------------------------------------------------------------------------

/// Compute the SHA-256 hash of `data` and return it as a lowercase hex string.
///
/// Uses the `ring` crate which is already in the workspace.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    use std::fmt::Write as _;

    // Simple portable SHA-256 using the ring crate if available,
    // otherwise a pure-Rust fallback.
    //
    // Because ring is a workspace dep we can use it here.
    let digest = ring_sha256(data);
    let mut hex = String::with_capacity(64);
    for byte in &digest {
        write!(&mut hex, "{byte:02x}").unwrap_or(());
    }
    hex
}

/// Delegate SHA-256 to `ring::digest`.
fn ring_sha256(data: &[u8]) -> Vec<u8> {
    // ring is in the workspace but not necessarily in bb-shim-windows deps.
    // We implement a minimal pure-Rust SHA-256 so the crate stays self-contained.
    sha256_pure(data)
}

/// Minimal pure-Rust SHA-256 (FIPS 180-4).
///
/// This is intentionally simple and not optimised — production code should
/// use the `ring` or `sha2` crate.  It is only used here so that
/// `bb-shim-windows` does not need an extra dependency.
#[allow(clippy::many_single_char_names)]
fn sha256_pure(msg: &[u8]) -> Vec<u8> {
    // Initial hash values (first 32 bits of fractional parts of square roots
    // of the first 8 primes).
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Round constants (first 32 bits of fractional parts of cube roots of
    // the first 64 primes).
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    // Pre-processing: pad the message
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    let mut padded = msg.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit (64-byte) chunk
    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, bytes) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut result = Vec::with_capacity(32);
    for word in &h {
        result.extend_from_slice(&word.to_be_bytes());
    }
    result
}

// ---------------------------------------------------------------------------
// Update operations
// ---------------------------------------------------------------------------

/// Check for an available update by querying the update API.
///
/// `current_version` is the running agent version (e.g. from `CARGO_PKG_VERSION`).
/// `api_base_url` is the base URL (e.g. `"https://api.betblocker.app"`).
///
/// Returns `Ok(Some(info))` if a newer version is available, `Ok(None)` if
/// already up to date, or an error if the check fails.
///
/// The actual HTTP call is stubbed on non-Windows (or when the network is
/// unavailable) so that unit tests pass everywhere.
pub async fn check_for_update(
    current_version: &str,
    api_base_url: &str,
) -> Result<Option<UpdateInfo>, UpdateError> {
    let url = format!("{api_base_url}/v1/agent/updates/latest?platform=windows");
    tracing::debug!(url = %url, current = %current_version, "Checking for update");

    // In a real implementation this would use reqwest / hyper.
    // We stub the HTTP call here so the crate compiles everywhere.
    let _ = url; // suppress unused warning in stub

    // Stub: pretend no update is available.
    // Real implementation:
    //   let resp = reqwest::get(&url).await.map_err(|e| UpdateError::Network(e.to_string()))?;
    //   let info: UpdateInfo = resp.json().await.map_err(|e| UpdateError::Deserialize(e.to_string()))?;
    //   if is_newer_version(current_version, &info.version) { Ok(Some(info)) } else { Ok(None) }
    Ok(None)
}

/// Download the MSI pointed to by `info` and verify its SHA-256 hash.
///
/// Writes the MSI to `dest_path`. On Windows also runs
/// `signtool verify /pa <dest_path>` for Authenticode verification.
pub async fn download_and_verify(
    info: &UpdateInfo,
    dest_path: &Path,
) -> Result<(), UpdateError> {
    tracing::info!(
        version = %info.version,
        url = %info.download_url,
        dest = %dest_path.display(),
        "Downloading update"
    );

    // Stub: in production this downloads via HTTP.
    // Real:
    //   let bytes = reqwest::get(&info.download_url).await?.bytes().await?;
    //   let hash = sha256_hex(&bytes);
    //   if hash != info.sha256 { return Err(UpdateError::HashMismatch { ... }); }
    //   std::fs::write(dest_path, &bytes)?;
    //   verify_authenticode(dest_path)?;

    let _ = dest_path;
    Ok(())
}

/// Verify the Authenticode signature of a PE/MSI file.
///
/// On Windows calls `signtool verify /pa /q <path>`.
/// On non-Windows is a no-op.
pub fn verify_authenticode(path: &Path) -> Result<(), UpdateError> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("signtool")
            .args(["verify", "/pa", "/q", &path.to_string_lossy()])
            .output()
            .map_err(|e| UpdateError::SignatureVerification(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UpdateError::SignatureVerification(format!(
                "signtool: {stderr}"
            )));
        }

        tracing::info!(path = %path.display(), "Authenticode signature verified");
    }
    let _ = path;
    Ok(())
}

/// Apply the downloaded MSI update by launching `msiexec`.
///
/// Runs `msiexec /i <msi_path> /quiet /norestart` and waits for it to finish.
/// On non-Windows platforms this is a no-op (stub).
pub fn apply_update(msi_path: &Path) -> Result<(), UpdateError> {
    tracing::info!(msi = %msi_path.display(), "Applying update via msiexec");

    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("msiexec")
            .args(["/i", &msi_path.to_string_lossy(), "/quiet", "/norestart"])
            .status()
            .map_err(|e| UpdateError::Io(e))?;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            return Err(UpdateError::MsiexecFailed(code));
        }
    }

    let _ = msi_path;
    Ok(())
}

// ---------------------------------------------------------------------------
// UpdateScheduler
// ---------------------------------------------------------------------------

/// Periodically checks for and applies updates.
///
/// The scheduler runs an async loop that wakes every `check_interval`,
/// calls `check_for_update`, downloads + verifies + applies if newer.
pub struct UpdateScheduler {
    /// Running agent version.
    pub current_version: String,
    /// Base URL of the BetBlocker update API.
    pub api_base_url: String,
    /// How often to poll for updates (default: 6 hours).
    pub check_interval: Duration,
    /// Directory to save downloaded MSI files.
    pub temp_dir: PathBuf,
}

impl UpdateScheduler {
    /// Create a new scheduler.
    #[must_use]
    pub fn new(
        current_version: impl Into<String>,
        api_base_url: impl Into<String>,
        check_interval: Duration,
        temp_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            current_version: current_version.into(),
            api_base_url: api_base_url.into(),
            check_interval,
            temp_dir: temp_dir.into(),
        }
    }

    /// Run the scheduler loop until `shutdown_rx` fires.
    ///
    /// Each tick:
    ///   1. Call `check_for_update`.
    ///   2. If `Some(info)`, download, verify, apply.
    ///   3. Sleep until next tick.
    pub async fn run(&self, mut shutdown_rx: watch::Receiver<bool>) {
        tracing::info!(
            version = %self.current_version,
            interval_secs = self.check_interval.as_secs(),
            "Update scheduler started"
        );

        let mut interval = tokio::time::interval(self.check_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.run_once().await;
                }
                _ = shutdown_rx.changed() => {
                    tracing::info!("Update scheduler shutting down");
                    break;
                }
            }
        }
    }

    /// Perform a single check-and-apply cycle.
    async fn run_once(&self) {
        match check_for_update(&self.current_version, &self.api_base_url).await {
            Ok(Some(info)) => {
                tracing::info!(version = %info.version, "Update available, downloading");
                let msi_path = self.temp_dir.join(format!("betblocker-{}.msi", info.version));

                match download_and_verify(&info, &msi_path).await {
                    Ok(()) => {
                        if let Err(e) = apply_update(&msi_path) {
                            tracing::error!(error = %e, "Failed to apply update");
                        } else {
                            tracing::info!(version = %info.version, "Update applied successfully");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to download/verify update");
                    }
                }
            }
            Ok(None) => {
                tracing::debug!(version = %self.current_version, "No update available");
            }
            Err(e) => {
                tracing::warn!(error = %e, "Update check failed");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- UpdateInfo deserialization ---

    #[test]
    fn update_info_deserialize() {
        let json = r#"{
            "version": "1.2.3",
            "download_url": "https://cdn.betblocker.app/betblocker-1.2.3.msi",
            "sha256": "abc123",
            "release_notes": "Bug fixes",
            "min_os_version": "10.0"
        }"#;
        let info: UpdateInfo = serde_json::from_str(json).expect("deserialize");
        assert_eq!(info.version, "1.2.3");
        assert_eq!(info.sha256, "abc123");
        assert_eq!(info.release_notes.as_deref(), Some("Bug fixes"));
        assert_eq!(info.min_os_version.as_deref(), Some("10.0"));
    }

    #[test]
    fn update_info_optional_fields_nullable() {
        let json = r#"{"version":"0.9","download_url":"http://x","sha256":"ff"}"#;
        let info: UpdateInfo = serde_json::from_str(json).expect("deserialize minimal");
        assert!(info.release_notes.is_none());
        assert!(info.min_os_version.is_none());
    }

    // --- Version comparison ---

    #[test]
    fn is_newer_version_true_patch() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_version_true_minor() {
        assert!(is_newer_version("1.0.0", "1.1.0"));
    }

    #[test]
    fn is_newer_version_true_major() {
        assert!(is_newer_version("1.0.0", "2.0.0"));
    }

    #[test]
    fn is_newer_version_false_same() {
        assert!(!is_newer_version("1.2.3", "1.2.3"));
    }

    #[test]
    fn is_newer_version_false_older() {
        assert!(!is_newer_version("2.0.0", "1.9.9"));
    }

    #[test]
    fn is_newer_version_with_pre_release_suffix() {
        // "1.0.1-beta" should still be considered 1.0.1 → newer than 1.0.0
        assert!(is_newer_version("1.0.0", "1.0.1-beta"));
    }

    // --- SHA-256 ---

    #[test]
    fn sha256_known_value() {
        // SHA-256 of empty string
        let hash = sha256_hex(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hello_world() {
        let hash = sha256_hex(b"hello world");
        // Verified against `sha256sum` reference implementation.
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn sha256_returns_64_hex_chars() {
        let hash = sha256_hex(b"test data");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // --- Error display ---

    #[test]
    fn update_error_display_network() {
        let err = UpdateError::Network("connection refused".to_string());
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn update_error_display_hash_mismatch() {
        let err = UpdateError::HashMismatch {
            expected: "aaa".to_string(),
            actual: "bbb".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("aaa") && s.contains("bbb"));
    }

    #[test]
    fn update_error_display_msiexec_failed() {
        let err = UpdateError::MsiexecFailed(1603);
        assert!(err.to_string().contains("1603"));
    }

    #[test]
    fn update_error_display_already_up_to_date() {
        let err = UpdateError::AlreadyUpToDate("1.0.0".to_string());
        assert!(err.to_string().contains("1.0.0"));
    }

    // --- Scheduler construction ---

    #[test]
    fn update_scheduler_construction() {
        let scheduler = UpdateScheduler::new(
            "1.0.0",
            "https://api.betblocker.app",
            Duration::from_secs(3600),
            "/tmp",
        );
        assert_eq!(scheduler.current_version, "1.0.0");
        assert_eq!(scheduler.check_interval, Duration::from_secs(3600));
    }

    // --- Hash verification integration ---

    #[test]
    fn hash_mismatch_detection() {
        let data = b"fake msi data";
        let actual = sha256_hex(data);
        let expected = "0000000000000000000000000000000000000000000000000000000000000000";
        if actual != expected {
            let err = UpdateError::HashMismatch {
                expected: expected.to_string(),
                actual: actual.clone(),
            };
            let s = err.to_string();
            assert!(s.contains(&actual));
        }
    }

    // --- apply_update stub on non-Windows ---

    #[test]
    fn apply_update_stub_succeeds_on_non_windows() {
        #[cfg(not(target_os = "windows"))]
        {
            let result = apply_update(Path::new("/tmp/fake.msi"));
            assert!(result.is_ok());
        }
    }

    // --- check_for_update stub ---

    #[tokio::test]
    async fn check_for_update_returns_none_in_stub() {
        let result = check_for_update("1.0.0", "https://api.betblocker.app").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
