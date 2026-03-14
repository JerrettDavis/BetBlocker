//! Windows minifilter driver IOCTL interface.
//!
//! Communicates with the `BetBlockerMF` minifilter kernel driver to protect
//! BetBlocker's own files from being tampered with while the service is
//! running.  Requires the `kernel-drivers` feature.
//!
//! # IOCTL code construction
//!
//! All codes use the standard Windows `CTL_CODE` macro:
//!
//! ```text
//! CTL_CODE(DeviceType, Function, Method, Access)
//!   = (DeviceType << 16) | (Access << 14) | (Function << 2) | Method
//! ```
//!
//! Constants:
//!   - `FILE_DEVICE_UNKNOWN` = 0x22
//!   - `METHOD_BUFFERED`     = 0
//!   - `FILE_ANY_ACCESS`     = 0
//!   - Function base (MF)    = 0x810  (separate range from WFP at 0x800)
//!
//! # Production usage
//!
//! Minifilter drivers expose a communication port rather than a device object.
//! A production implementation would call `FilterConnectCommunicationPort`
//! (from `fltLib.dll`) to connect, then `FilterSendMessage` /
//! `FilterGetMessage` to exchange data.  Those APIs require linking against
//! `FltLib.lib` which is a Wave-2 build dependency.
//!
//! For now, the `open()` constructor returns a mock client so that the rest
//! of the codebase can compile and test without the filter library.

use std::io;

use thiserror::Error;

// ---------------------------------------------------------------------------
// IOCTL code constants
// ---------------------------------------------------------------------------

/// Construct a CTL_CODE value at compile time.
const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const FILE_DEVICE_UNKNOWN: u32 = 0x22;
const METHOD_BUFFERED: u32 = 0;
const FILE_ANY_ACCESS: u32 = 0;

/// Query whether the minifilter is loaded and how many paths it protects.
pub const IOCTL_MF_GET_STATUS: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x810, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Register a filesystem path for protection.
pub const IOCTL_MF_ADD_PROTECTED_PATH: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x811, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Unregister a previously protected path.
pub const IOCTL_MF_REMOVE_PROTECTED_PATH: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x812, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Provide the 32-byte HMAC token the driver uses to authenticate self-updates.
pub const IOCTL_MF_SET_UPDATE_TOKEN: u32 =
    ctl_code(FILE_DEVICE_UNKNOWN, 0x813, METHOD_BUFFERED, FILE_ANY_ACCESS);

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MinifilterError {
    #[error("failed to open minifilter communication port: {0}")]
    DeviceOpen(#[source] io::Error),

    #[error("filter message failed (code 0x{code:08X}): {source}")]
    Ioctl {
        code: u32,
        #[source]
        source: io::Error,
    },

    #[error("response buffer too small: expected {expected} bytes, got {got}")]
    BufferTooSmall { expected: usize, got: usize },

    #[error("path string contains interior NUL byte")]
    InvalidPath,

    #[error("update token must be exactly 32 bytes")]
    InvalidToken,
}

pub type Result<T, E = MinifilterError> = std::result::Result<T, E>;

// ---------------------------------------------------------------------------
// Status structure
// ---------------------------------------------------------------------------

/// Runtime status reported by the minifilter driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinifilterStatus {
    /// Whether the filter is currently attached and active.
    pub active: bool,
    /// Number of paths currently under protection.
    pub protected_paths: u32,
    /// Cumulative count of blocked filesystem operations since driver load.
    pub blocked_operations: u64,
}

impl MinifilterStatus {
    /// Wire layout (little-endian, from C driver):
    ///   `[0]`     `active`            (u8, 0 or 1)
    ///   `[1..4]`  padding             (zeroed)
    ///   `[4..8]`  `protected_paths`   (u32 LE)
    ///   `[8..16]` `blocked_operations`(u64 LE)
    const WIRE_SIZE: usize = 16;

    fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::WIRE_SIZE {
            return Err(MinifilterError::BufferTooSmall {
                expected: Self::WIRE_SIZE,
                got: buf.len(),
            });
        }
        let active = buf[0] != 0;
        // bytes 1..4 padding
        let protected_paths =
            u32::from_le_bytes(buf[4..8].try_into().expect("slice is 4 bytes"));
        let blocked_operations =
            u64::from_le_bytes(buf[8..16].try_into().expect("slice is 8 bytes"));
        Ok(Self {
            active,
            protected_paths,
            blocked_operations,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal mock handle
// ---------------------------------------------------------------------------

/// Opaque handle to the minifilter communication port.
///
/// A production implementation would store the port handle returned by
/// `FilterConnectCommunicationPort` and call `FilterSendMessage` /
/// `FilterGetMessage` in `ioctl()`.  Those Win32 calls require `unsafe`
/// blocks; they are deferred to Wave 2 to keep the current code
/// `unsafe`-free (matching the workspace lint `unsafe_code = "deny"`).
enum DeviceHandle {
    /// Stub used in tests and until the real driver is available.
    Mock,
}

impl DeviceHandle {
    fn ioctl(&self, code: u32, input: &[u8]) -> Result<Vec<u8>> {
        match self {
            DeviceHandle::Mock => {
                tracing::debug!(
                    ioctl_code = code,
                    input_len = input.len(),
                    "MF IOCTL (mock — no driver present)"
                );
                Ok(Vec::new())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for communicating with the BetBlockerMF minifilter kernel driver.
pub struct MinifilterClient {
    handle: DeviceHandle,
}

impl MinifilterClient {
    /// Communication-port name exposed by the minifilter driver.
    pub const PORT_NAME: &'static str = r"\\BetBlockerMFPort";

    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Open a connection to the minifilter driver communication port.
    ///
    /// Until the Wave-2 kernel driver is available this always returns a mock
    /// client.  On a system where the driver is installed, a production
    /// implementation would call `FilterConnectCommunicationPort(PORT_NAME,
    /// ...)`.
    pub fn open() -> Result<Self> {
        tracing::warn!(
            port = Self::PORT_NAME,
            "MinifilterClient::open() — driver not yet available, using mock handle"
        );
        Ok(Self {
            handle: DeviceHandle::Mock,
        })
    }

    /// Create a client backed by a mock handle (no driver required).
    pub fn open_mock() -> Self {
        Self {
            handle: DeviceHandle::Mock,
        }
    }

    // -----------------------------------------------------------------------
    // Generic IOCTL helper
    // -----------------------------------------------------------------------

    /// Send an IOCTL-style message to the driver and return the response bytes.
    pub fn send_ioctl(&self, code: u32, input: &[u8]) -> Result<Vec<u8>> {
        self.handle.ioctl(code, input)
    }

    // -----------------------------------------------------------------------
    // Typed API
    // -----------------------------------------------------------------------

    /// Register a filesystem path for minifilter protection.
    ///
    /// `path` should be an NT-namespace absolute path, e.g.
    /// `\Device\HarddiskVolume3\Program Files\BetBlocker`.
    pub fn add_protected_path(&self, path: &str) -> Result<()> {
        if path.contains('\0') {
            return Err(MinifilterError::InvalidPath);
        }
        self.send_ioctl(IOCTL_MF_ADD_PROTECTED_PATH, path.as_bytes())?;
        tracing::info!(path, "MF: added protected path");
        Ok(())
    }

    /// Remove a previously registered protected path.
    pub fn remove_protected_path(&self, path: &str) -> Result<()> {
        if path.contains('\0') {
            return Err(MinifilterError::InvalidPath);
        }
        self.send_ioctl(IOCTL_MF_REMOVE_PROTECTED_PATH, path.as_bytes())?;
        tracing::info!(path, "MF: removed protected path");
        Ok(())
    }

    /// Query driver status.
    pub fn get_status(&self) -> Result<MinifilterStatus> {
        let buf = self.send_ioctl(IOCTL_MF_GET_STATUS, &[])?;
        if buf.is_empty() {
            // Mock path: return a sensible default.
            return Ok(MinifilterStatus {
                active: false,
                protected_paths: 0,
                blocked_operations: 0,
            });
        }
        MinifilterStatus::from_bytes(&buf)
    }

    /// Provide the 32-byte HMAC token used to authenticate self-update writes.
    ///
    /// The minifilter driver will permit writes to protected paths only when
    /// the caller presents a valid token.
    pub fn set_update_token(&self, token: &[u8; 32]) -> Result<()> {
        self.send_ioctl(IOCTL_MF_SET_UPDATE_TOKEN, token.as_slice())?;
        tracing::info!("MF: update token set");
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

    #[test]
    fn ioctl_get_status_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x810u32 << 2);
        assert_eq!(IOCTL_MF_GET_STATUS, expected);
        assert_eq!(IOCTL_MF_GET_STATUS, 0x0022_2040);
    }

    #[test]
    fn ioctl_add_protected_path_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x811u32 << 2);
        assert_eq!(IOCTL_MF_ADD_PROTECTED_PATH, expected);
        assert_eq!(IOCTL_MF_ADD_PROTECTED_PATH, 0x0022_2044);
    }

    #[test]
    fn ioctl_remove_protected_path_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x812u32 << 2);
        assert_eq!(IOCTL_MF_REMOVE_PROTECTED_PATH, expected);
        assert_eq!(IOCTL_MF_REMOVE_PROTECTED_PATH, 0x0022_2048);
    }

    #[test]
    fn ioctl_set_update_token_value() {
        let expected: u32 = (FILE_DEVICE_UNKNOWN << 16) | (0x813u32 << 2);
        assert_eq!(IOCTL_MF_SET_UPDATE_TOKEN, expected);
        assert_eq!(IOCTL_MF_SET_UPDATE_TOKEN, 0x0022_204C);
    }

    #[test]
    fn ioctl_codes_are_distinct() {
        let codes = [
            IOCTL_MF_GET_STATUS,
            IOCTL_MF_ADD_PROTECTED_PATH,
            IOCTL_MF_REMOVE_PROTECTED_PATH,
            IOCTL_MF_SET_UPDATE_TOKEN,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j], "IOCTL codes[{i}] and codes[{j}] collide");
            }
        }
    }

    // -----------------------------------------------------------------------
    // MinifilterStatus deserialisation
    // -----------------------------------------------------------------------

    #[test]
    fn status_deserialise_active() {
        let mut buf = [0u8; 16];
        buf[0] = 1; // active = true
        buf[4..8].copy_from_slice(&5u32.to_le_bytes()); // protected_paths = 5
        buf[8..16].copy_from_slice(&99u64.to_le_bytes()); // blocked_operations = 99

        let status = MinifilterStatus::from_bytes(&buf).unwrap();
        assert!(status.active);
        assert_eq!(status.protected_paths, 5);
        assert_eq!(status.blocked_operations, 99);
    }

    #[test]
    fn status_deserialise_inactive() {
        let buf = [0u8; 16]; // all zeros → inactive
        let status = MinifilterStatus::from_bytes(&buf).unwrap();
        assert!(!status.active);
        assert_eq!(status.protected_paths, 0);
        assert_eq!(status.blocked_operations, 0);
    }

    #[test]
    fn status_buffer_too_small_returns_error() {
        let buf = [0u8; 8];
        assert!(MinifilterStatus::from_bytes(&buf).is_err());
    }

    // -----------------------------------------------------------------------
    // Mock-handle round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn add_remove_path_via_mock() {
        let client = MinifilterClient::open_mock();
        client
            .add_protected_path(r"\Device\HarddiskVolume3\Program Files\BetBlocker")
            .unwrap();
        client
            .remove_protected_path(r"\Device\HarddiskVolume3\Program Files\BetBlocker")
            .unwrap();
    }

    #[test]
    fn path_with_nul_is_rejected() {
        let client = MinifilterClient::open_mock();
        assert!(client.add_protected_path("bad\0path").is_err());
        assert!(client.remove_protected_path("bad\0path").is_err());
    }

    #[test]
    fn get_status_returns_default_for_mock() {
        let client = MinifilterClient::open_mock();
        let status = client.get_status().unwrap();
        assert!(!status.active);
        assert_eq!(status.protected_paths, 0);
        assert_eq!(status.blocked_operations, 0);
    }

    #[test]
    fn set_update_token_accepts_32_bytes() {
        let client = MinifilterClient::open_mock();
        let token = [0xABu8; 32];
        client.set_update_token(&token).unwrap();
    }
}
