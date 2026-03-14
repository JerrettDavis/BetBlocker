//! Windows Filtering Platform (WFP) IOCTL interface.
//!
//! Provides kernel-level network filtering for DNS redirection and traffic
//! interception by communicating with the `BetBlockerWfp` kernel driver via
//! DeviceIoControl.  Requires the `kernel-drivers` feature.
//!
//! # IOCTL code construction
//!
//! Every code follows the standard Windows `CTL_CODE` macro:
//!
//! ```text
//! CTL_CODE(DeviceType, Function, Method, Access)
//!   = (DeviceType << 16) | (Access << 14) | (Function << 2) | Method
//! ```
//!
//! Constants used here:
//!   - `FILE_DEVICE_UNKNOWN` = 0x22
//!   - `METHOD_BUFFERED`     = 0
//!   - `FILE_ANY_ACCESS`     = 0
//!   - Function base         = 0x800 (user-defined range starts at 0x800)
//!
//! # Production usage
//!
//! The Windows-only `open()` constructor would call `CreateFileW` to open
//! `\\.\BetBlockerWfp`, and `send_ioctl` would call `DeviceIoControl`.
//! Those Win32 API calls require the `Win32_Storage_FileSystem` and
//! `Win32_System_IO` windows-crate features, which are left as a TODO in the
//! `Cargo.toml` until the kernel-driver build environment is set up.
//!
//! For now, the `open()` constructor falls back to a mock handle on all
//! platforms, allowing the rest of the codebase to compile and test.

use std::io;

use thiserror::Error;

// ---------------------------------------------------------------------------
// IOCTL code constants
// ---------------------------------------------------------------------------

/// Construct a CTL_CODE value at compile time.
///
/// `CTL_CODE(DeviceType, Function, Method, Access)`
const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const FILE_DEVICE_UNKNOWN: u32 = 0x22;
const METHOD_BUFFERED: u32 = 0;
const FILE_ANY_ACCESS: u32 = 0;

/// Add a domain to the WFP block list.
pub const IOCTL_WFP_ADD_BLOCKED_DOMAIN: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Remove a domain from the WFP block list.
pub const IOCTL_WFP_REMOVE_BLOCKED_DOMAIN: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Clear all entries from the WFP block list.
pub const IOCTL_WFP_CLEAR_BLOCKLIST: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Retrieve runtime statistics from the WFP driver.
pub const IOCTL_WFP_GET_STATS: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x803, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Configure the DNS-redirect port used by the callout.
pub const IOCTL_WFP_SET_DNS_REDIRECT: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x804, METHOD_BUFFERED, FILE_ANY_ACCESS);

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum WfpError {
    #[error("failed to open WFP driver device: {0}")]
    DeviceOpen(#[source] io::Error),

    #[error("DeviceIoControl failed (code 0x{code:08X}): {source}")]
    Ioctl {
        code: u32,
        #[source]
        source: io::Error,
    },

    #[error("response buffer too small: expected {expected} bytes, got {got}")]
    BufferTooSmall { expected: usize, got: usize },

    #[error("domain string contains interior NUL byte")]
    InvalidDomain,
}

pub type Result<T, E = WfpError> = std::result::Result<T, E>;

// ---------------------------------------------------------------------------
// Stats structure
// ---------------------------------------------------------------------------

/// Runtime statistics reported by the WFP callout driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WfpStats {
    /// Total DNS queries blocked since driver load.
    pub blocked_queries: u64,
    /// Number of active block-list rules.
    pub active_rules: u32,
    /// Seconds elapsed since the driver was loaded.
    pub uptime_secs: u64,
}

impl WfpStats {
    /// Expected size of the little-endian wire encoding produced by the C driver.
    ///
    /// Layout:
    ///   `[0..8]`   `blocked_queries` (u64 LE)
    ///   `[8..12]`  `active_rules`    (u32 LE)
    ///   `[12..16]` padding            (zeroed)
    ///   `[16..24]` `uptime_secs`      (u64 LE)
    const WIRE_SIZE: usize = 24;

    /// Deserialize from the little-endian wire format produced by the kernel driver.
    fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::WIRE_SIZE {
            return Err(WfpError::BufferTooSmall {
                expected: Self::WIRE_SIZE,
                got: buf.len(),
            });
        }
        let blocked_queries =
            u64::from_le_bytes(buf[0..8].try_into().expect("slice is 8 bytes"));
        let active_rules =
            u32::from_le_bytes(buf[8..12].try_into().expect("slice is 4 bytes"));
        // bytes 12..16 are padding
        let uptime_secs =
            u64::from_le_bytes(buf[16..24].try_into().expect("slice is 8 bytes"));
        Ok(Self {
            blocked_queries,
            active_rules,
            uptime_secs,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal mock handle
// ---------------------------------------------------------------------------

/// Opaque device-handle representation.
///
/// Production Windows code would store a Win32 `HANDLE` here and call
/// `DeviceIoControl` inside `ioctl()`.  All current callers use the `Mock`
/// variant because the kernel driver is a Wave-2 deliverable.
///
/// The Win32 implementation is intentionally left as a documented TODO to
/// avoid pulling in `unsafe` blocks before the full WDK build is configured
/// (see `Cargo.toml` `kernel-drivers` feature notes).
enum DeviceHandle {
    /// Stub used in tests and until the real driver is available.
    Mock,
}

impl DeviceHandle {
    /// Send an IOCTL to the underlying device and return the output buffer.
    fn ioctl(&self, code: u32, input: &[u8]) -> Result<Vec<u8>> {
        match self {
            DeviceHandle::Mock => {
                tracing::debug!(
                    ioctl_code = code,
                    input_len = input.len(),
                    "WFP IOCTL (mock — no driver present)"
                );
                Ok(Vec::new())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for communicating with the BetBlockerWfp kernel driver.
pub struct WfpDriverClient {
    handle: DeviceHandle,
}

impl WfpDriverClient {
    /// Device path that `CreateFileW` would use on a production system.
    pub const DEVICE_PATH: &'static str = r"\\.\BetBlockerWfp";

    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Open a connection to the WFP driver device.
    ///
    /// Until the Wave-2 kernel driver is available this always returns a mock
    /// client.  On a system where the driver is installed, a production
    /// implementation would call `CreateFileW(DEVICE_PATH, ...)`.
    pub fn open() -> Result<Self> {
        tracing::warn!(
            device = Self::DEVICE_PATH,
            "WfpDriverClient::open() — driver not yet available, using mock handle"
        );
        Ok(Self {
            handle: DeviceHandle::Mock,
        })
    }

    /// Create a client backed by a mock handle (no driver required).
    ///
    /// Intended for unit tests and CI environments.
    pub fn open_mock() -> Self {
        Self {
            handle: DeviceHandle::Mock,
        }
    }

    // -----------------------------------------------------------------------
    // Generic IOCTL helper
    // -----------------------------------------------------------------------

    /// Send an IOCTL code with an arbitrary byte payload and return the
    /// response bytes.
    pub fn send_ioctl(&self, code: u32, input: &[u8]) -> Result<Vec<u8>> {
        self.handle.ioctl(code, input)
    }

    // -----------------------------------------------------------------------
    // Typed API
    // -----------------------------------------------------------------------

    /// Add `domain` to the kernel block list.
    ///
    /// The domain is encoded as a UTF-8 byte string (no NUL terminator) in the
    /// IOCTL input buffer — matching what the C driver expects.
    pub fn add_blocked_domain(&self, domain: &str) -> Result<()> {
        if domain.contains('\0') {
            return Err(WfpError::InvalidDomain);
        }
        self.send_ioctl(IOCTL_WFP_ADD_BLOCKED_DOMAIN, domain.as_bytes())?;
        tracing::info!(domain, "WFP: added blocked domain");
        Ok(())
    }

    /// Remove `domain` from the kernel block list.
    pub fn remove_blocked_domain(&self, domain: &str) -> Result<()> {
        if domain.contains('\0') {
            return Err(WfpError::InvalidDomain);
        }
        self.send_ioctl(IOCTL_WFP_REMOVE_BLOCKED_DOMAIN, domain.as_bytes())?;
        tracing::info!(domain, "WFP: removed blocked domain");
        Ok(())
    }

    /// Clear all entries from the kernel block list.
    pub fn clear_blocklist(&self) -> Result<()> {
        self.send_ioctl(IOCTL_WFP_CLEAR_BLOCKLIST, &[])?;
        tracing::info!("WFP: blocklist cleared");
        Ok(())
    }

    /// Query runtime statistics from the driver.
    ///
    /// Returns zeroed stats when the mock handle is in use.
    pub fn get_stats(&self) -> Result<WfpStats> {
        let buf = self.send_ioctl(IOCTL_WFP_GET_STATS, &[])?;
        if buf.is_empty() {
            // Mock path: return zeroed stats.
            return Ok(WfpStats {
                blocked_queries: 0,
                active_rules: 0,
                uptime_secs: 0,
            });
        }
        WfpStats::from_bytes(&buf)
    }

    /// Configure the UDP port to which blocked DNS queries are redirected.
    pub fn set_dns_redirect(&self, port: u16) -> Result<()> {
        let buf = port.to_le_bytes();
        self.send_ioctl(IOCTL_WFP_SET_DNS_REDIRECT, &buf)?;
        tracing::info!(port, "WFP: DNS redirect port set");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // IOCTL code bit-pattern verification
    // -----------------------------------------------------------------------

    /// Validate that `ctl_code` produces the expected bit pattern.
    ///
    /// Reference: `CTL_CODE(0x22, 0x800, 0, 0)` should equal
    /// `(0x22 << 16) | (0 << 14) | (0x800 << 2) | 0`
    /// = `0x0022_0000 | 0x0000_2000` = `0x0022_2000`
    #[test]
    fn ioctl_add_blocked_domain_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16)
            | (FILE_ANY_ACCESS << 14)
            | (0x800u32 << 2)
            | METHOD_BUFFERED;
        assert_eq!(IOCTL_WFP_ADD_BLOCKED_DOMAIN, expected);
        assert_eq!(IOCTL_WFP_ADD_BLOCKED_DOMAIN, 0x0022_2000);
    }

    #[test]
    fn ioctl_remove_blocked_domain_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x801u32 << 2);
        assert_eq!(IOCTL_WFP_REMOVE_BLOCKED_DOMAIN, expected);
        assert_eq!(IOCTL_WFP_REMOVE_BLOCKED_DOMAIN, 0x0022_2004);
    }

    #[test]
    fn ioctl_clear_blocklist_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x802u32 << 2);
        assert_eq!(IOCTL_WFP_CLEAR_BLOCKLIST, expected);
        assert_eq!(IOCTL_WFP_CLEAR_BLOCKLIST, 0x0022_2008);
    }

    #[test]
    fn ioctl_get_stats_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x803u32 << 2);
        assert_eq!(IOCTL_WFP_GET_STATS, expected);
        assert_eq!(IOCTL_WFP_GET_STATS, 0x0022_200C);
    }

    #[test]
    fn ioctl_set_dns_redirect_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x804u32 << 2);
        assert_eq!(IOCTL_WFP_SET_DNS_REDIRECT, expected);
        assert_eq!(IOCTL_WFP_SET_DNS_REDIRECT, 0x0022_2010);
    }

    #[test]
    fn ioctl_codes_are_distinct() {
        let codes = [
            IOCTL_WFP_ADD_BLOCKED_DOMAIN,
            IOCTL_WFP_REMOVE_BLOCKED_DOMAIN,
            IOCTL_WFP_CLEAR_BLOCKLIST,
            IOCTL_WFP_GET_STATS,
            IOCTL_WFP_SET_DNS_REDIRECT,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j], "IOCTL codes[{i}] and codes[{j}] collide");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Domain serialisation
    // -----------------------------------------------------------------------

    #[test]
    fn domain_encodes_as_utf8_bytes() {
        let domain = "gambling-example.com";
        let bytes = domain.as_bytes();
        assert_eq!(bytes, b"gambling-example.com");
        assert!(!bytes.contains(&0u8), "no NUL in encoded domain");
    }

    #[test]
    fn domain_with_nul_is_rejected() {
        let client = WfpDriverClient::open_mock();
        assert!(client.add_blocked_domain("bad\0domain").is_err());
        assert!(client.remove_blocked_domain("bad\0domain").is_err());
    }

    #[test]
    fn add_remove_domain_via_mock() {
        let client = WfpDriverClient::open_mock();
        client.add_blocked_domain("bet365.com").unwrap();
        client.remove_blocked_domain("bet365.com").unwrap();
        client.clear_blocklist().unwrap();
    }

    // -----------------------------------------------------------------------
    // WfpStats deserialisation
    // -----------------------------------------------------------------------

    #[test]
    fn stats_deserialise_little_endian() {
        // blocked_queries = 1000, active_rules = 42, uptime_secs = 3600
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&1000u64.to_le_bytes());
        buf[8..12].copy_from_slice(&42u32.to_le_bytes());
        // bytes 12..16 = padding (already zero)
        buf[16..24].copy_from_slice(&3600u64.to_le_bytes());

        let stats = WfpStats::from_bytes(&buf).unwrap();
        assert_eq!(stats.blocked_queries, 1000);
        assert_eq!(stats.active_rules, 42);
        assert_eq!(stats.uptime_secs, 3600);
    }

    #[test]
    fn stats_buffer_too_small_returns_error() {
        let buf = [0u8; 10];
        assert!(WfpStats::from_bytes(&buf).is_err());
    }

    #[test]
    fn get_stats_returns_zeroed_for_mock() {
        let client = WfpDriverClient::open_mock();
        let stats = client.get_stats().unwrap();
        assert_eq!(stats.blocked_queries, 0);
        assert_eq!(stats.active_rules, 0);
        assert_eq!(stats.uptime_secs, 0);
    }

    #[test]
    fn set_dns_redirect_encodes_port_le() {
        // Verify port 5353 serialises to the right two bytes.
        let port: u16 = 5353;
        let bytes = port.to_le_bytes();
        assert_eq!(bytes, [0xE9, 0x14]); // 5353 = 0x14E9, little-endian → [0xE9, 0x14]
        // Smoke-test via mock client.
        let client = WfpDriverClient::open_mock();
        client.set_dns_redirect(5353).unwrap();
    }
}
