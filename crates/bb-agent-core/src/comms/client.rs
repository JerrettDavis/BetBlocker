use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use prost::Message;
use reqwest::{Certificate, Client, Identity, StatusCode};
use tokio::sync::RwLock;

/// Central HTTP client for all agent-to-API communication.
///
/// Uses mTLS with certificate pinning and protobuf serialization.
/// Includes retry with exponential backoff and a circuit breaker
/// to avoid hammering a down server.
pub struct ApiClient {
    /// Base URL of the BetBlocker API (e.g., "https://api.betblocker.org")
    base_url: String,
    /// reqwest client configured with mTLS identity and pinned CA
    client: Client,
    /// Device ID assigned during registration (None before registration)
    device_id: RwLock<Option<String>>,
    /// Retry configuration
    retry_config: RetryConfig,
    /// Circuit breaker state
    circuit: CircuitBreaker,
}

/// Retry configuration for API requests.
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(300),
            backoff_multiplier: 2.0,
        }
    }
}

/// Circuit breaker prevents repeated requests to a failing server.
///
/// States:
/// - Closed: requests flow normally.
/// - Open: all requests fail immediately for `open_duration`.
/// - HalfOpen: one probe request is allowed; success closes, failure re-opens.
struct CircuitBreaker {
    /// 0 = Closed, 1 = Open, 2 = HalfOpen
    state: AtomicU32,
    consecutive_failures: AtomicU32,
    failure_threshold: u32,
    /// Epoch millis when the circuit was opened
    opened_at: AtomicU64,
    open_duration: Duration,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32, open_duration: Duration) -> Self {
        Self {
            state: AtomicU32::new(0),
            consecutive_failures: AtomicU32::new(0),
            failure_threshold,
            opened_at: AtomicU64::new(0),
            open_duration,
        }
    }

    fn is_open(&self) -> bool {
        self.state.load(Ordering::Acquire) == 1
    }

    fn check_open_expiry(&self) -> bool {
        if !self.is_open() {
            return false;
        }
        let opened = self.opened_at.load(Ordering::Acquire);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if now.saturating_sub(opened) >= self.open_duration.as_millis() as u64 {
            // Transition to half-open
            self.state.store(2, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Returns true if the request should be allowed.
    fn should_allow(&self) -> bool {
        match self.state.load(Ordering::Acquire) {
            0 => true, // Closed
            1 => {
                // Open: check if duration has passed
                self.check_open_expiry()
            }
            2 => true, // HalfOpen: allow one probe
            _ => true,
        }
    }

    fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
        self.state.store(0, Ordering::Release);
    }

    fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        if failures >= self.failure_threshold {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.opened_at.store(now, Ordering::Release);
            self.state.store(1, Ordering::Release);
            tracing::warn!(
                failures,
                "Circuit breaker opened after {} consecutive failures",
                failures
            );
        }
    }
}

/// Errors from the API client.
#[derive(Debug, thiserror::Error)]
pub enum ApiClientError {
    #[error("Certificate error: {0}")]
    CertificateError(reqwest::Error),

    #[error("Identity error: {0}")]
    IdentityError(reqwest::Error),

    #[error("HTTP client build error: {0}")]
    HttpClientError(reqwest::Error),

    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Circuit breaker open -- server is unreachable")]
    CircuitBreakerOpen,

    #[error("Protobuf encode error: {0}")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Protobuf decode error: {0}")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Server returned {status}: {body}")]
    ServerError { status: StatusCode, body: String },

    #[error("Max retries ({0}) exceeded")]
    MaxRetriesExceeded(u32),

    #[error("Not registered (no device_id)")]
    NotRegistered,
}

impl ApiClient {
    /// Create a new `ApiClient` with mTLS.
    ///
    /// `device_identity` is `None` before initial registration (uses enrollment-only TLS).
    /// After registration, reconstruct with the device certificate + key (PEM-encoded).
    pub fn new(
        base_url: String,
        ca_cert_pem: &[u8],
        device_identity_pem: Option<&[u8]>,
        retry_config: RetryConfig,
    ) -> Result<Self, ApiClientError> {
        let ca_cert =
            Certificate::from_pem(ca_cert_pem).map_err(ApiClientError::CertificateError)?;

        let mut builder = Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(false) // Pin only our CA
            .add_root_certificate(ca_cert)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        if let Some(identity_bytes) = device_identity_pem {
            let identity =
                Identity::from_pem(identity_bytes).map_err(ApiClientError::IdentityError)?;
            builder = builder.identity(identity);
        }

        let client = builder.build().map_err(ApiClientError::HttpClientError)?;

        Ok(Self {
            base_url,
            client,
            device_id: RwLock::new(None),
            retry_config,
            circuit: CircuitBreaker::new(3, Duration::from_secs(60)),
        })
    }

    /// Create a minimal client for testing (no TLS configuration).
    #[cfg(test)]
    pub fn new_for_test(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url,
            client,
            device_id: RwLock::new(None),
            retry_config: RetryConfig::default(),
            circuit: CircuitBreaker::new(3, Duration::from_secs(60)),
        }
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Set the device ID after registration.
    pub async fn set_device_id(&self, id: String) {
        let mut guard = self.device_id.write().await;
        *guard = Some(id);
    }

    /// Get the current device ID.
    pub async fn device_id(&self) -> Option<String> {
        self.device_id.read().await.clone()
    }

    /// Send a POST request with protobuf body and retry logic.
    ///
    /// Serializes `request` as protobuf, sends to `{base_url}{path}`,
    /// deserializes the response as protobuf type `Resp`.
    pub async fn post_proto<Req: Message, Resp: Message + Default>(
        &self,
        path: &str,
        request: &Req,
    ) -> Result<Resp, ApiClientError> {
        if !self.circuit.should_allow() {
            return Err(ApiClientError::CircuitBreakerOpen);
        }

        let body = request.encode_to_vec();
        let url = format!("{}{path}", self.base_url);

        let mut attempt = 0u32;
        let mut backoff = self.retry_config.initial_backoff;

        loop {
            let result = self
                .client
                .post(&url)
                .header("content-type", "application/protobuf")
                .header("accept", "application/protobuf")
                .body(body.clone())
                .send()
                .await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        self.circuit.record_success();
                        let bytes = response.bytes().await?;
                        let resp = Resp::decode(bytes.as_ref())?;
                        return Ok(resp);
                    }

                    // Retry on 5xx and 429
                    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                        self.circuit.record_failure();
                        attempt += 1;
                        if attempt > self.retry_config.max_retries {
                            return Err(ApiClientError::MaxRetriesExceeded(
                                self.retry_config.max_retries,
                            ));
                        }

                        // Respect Retry-After header if present
                        if let Some(retry_after) = response.headers().get("retry-after") {
                            if let Ok(secs_str) = retry_after.to_str() {
                                if let Ok(secs) = secs_str.parse::<u64>() {
                                    let wait = Duration::from_secs(secs);
                                    tracing::debug!(
                                        attempt,
                                        wait_secs = secs,
                                        "Retrying after Retry-After header"
                                    );
                                    tokio::time::sleep(wait).await;
                                    continue;
                                }
                            }
                        }

                        tracing::debug!(
                            attempt,
                            status = %status,
                            backoff_ms = backoff.as_millis(),
                            "Retrying request"
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = Duration::from_secs_f64(
                            (backoff.as_secs_f64() * self.retry_config.backoff_multiplier)
                                .min(self.retry_config.max_backoff.as_secs_f64()),
                        );
                        continue;
                    }

                    // Non-retryable error (4xx except 429)
                    let body_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "<unreadable>".to_string());
                    return Err(ApiClientError::ServerError {
                        status,
                        body: body_text,
                    });
                }
                Err(e) => {
                    self.circuit.record_failure();
                    attempt += 1;
                    if attempt > self.retry_config.max_retries {
                        return Err(ApiClientError::MaxRetriesExceeded(
                            self.retry_config.max_retries,
                        ));
                    }
                    tracing::debug!(
                        attempt,
                        error = %e,
                        backoff_ms = backoff.as_millis(),
                        "Request failed, retrying"
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = Duration::from_secs_f64(
                        (backoff.as_secs_f64() * self.retry_config.backoff_multiplier)
                            .min(self.retry_config.max_backoff.as_secs_f64()),
                    );
                }
            }
        }
    }

    /// Send a raw POST with bytes body and get raw bytes back.
    /// Used for endpoints that may not use protobuf (e.g. certificate rotation).
    pub async fn post_raw(
        &self,
        path: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, ApiClientError> {
        if !self.circuit.should_allow() {
            return Err(ApiClientError::CircuitBreakerOpen);
        }

        let url = format!("{}{path}", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("content-type", content_type)
            .body(body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            self.circuit.record_success();
            let bytes = response.bytes().await?;
            Ok(bytes.to_vec())
        } else {
            self.circuit.record_failure();
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".to_string());
            Err(ApiClientError::ServerError {
                status,
                body: body_text,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff, Duration::from_secs(1));
        assert_eq!(config.max_backoff, Duration::from_secs(300));
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        assert!(!cb.is_open());
        assert!(!cb.is_open());
        assert!(cb.should_allow());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));

        cb.record_failure();
        assert!(!cb.is_open());
        cb.record_failure();
        assert!(!cb.is_open());
        cb.record_failure();
        // Now it should be open
        assert!(cb.is_open());
        assert!(!cb.should_allow());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));

        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // Reset
        assert!(!cb.is_open());

        // Should need 3 more failures to open
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_open());
    }

    #[tokio::test]
    async fn test_api_client_circuit_breaker_rejects() {
        let client = ApiClient::new_for_test("http://localhost:1".to_string());

        // Force circuit open
        client.circuit.record_failure();
        client.circuit.record_failure();
        client.circuit.record_failure();

        let result = client
            .post_proto::<bb_proto::heartbeat::HeartbeatRequest, bb_proto::heartbeat::HeartbeatResponse>(
                "/test",
                &bb_proto::heartbeat::HeartbeatRequest::default(),
            )
            .await;

        assert!(matches!(result, Err(ApiClientError::CircuitBreakerOpen)));
    }

    #[tokio::test]
    async fn test_api_client_set_device_id() {
        let client = ApiClient::new_for_test("http://localhost:1".to_string());
        assert!(client.device_id().await.is_none());

        client.set_device_id("test-device-123".to_string()).await;
        assert_eq!(
            client.device_id().await,
            Some("test-device-123".to_string())
        );
    }
}
