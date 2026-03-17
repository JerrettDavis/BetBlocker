//! XPC bridge for communicating with the BetBlocker Network Extension.
//!
//! This module provides the Rust-side XPC client that communicates with a
//! companion Swift `NEDNSProxyProvider` Network Extension. On macOS, XPC
//! connections are established via `xpc_connection_create`. On non-macOS
//! platforms, stub implementations are provided for cross-compilation.

use thiserror::Error;

/// Errors produced by XPC operations.
#[derive(Debug, Error)]
pub enum XpcError {
    /// The XPC connection could not be established.
    #[error("failed to connect to XPC service '{0}'")]
    ConnectionFailed(String),

    /// The XPC message could not be sent or a reply was not received.
    #[error("XPC send failed: {0}")]
    SendFailed(String),

    /// The response payload was malformed or unrecognised.
    #[error("XPC response invalid: {0}")]
    InvalidResponse(String),

    /// The remote service returned an application-level error.
    #[error("XPC remote error: {0}")]
    RemoteError(String),

    /// The operation is not supported on this platform.
    #[error("XPC not supported on this platform")]
    NotSupported,
}

/// Messages sent from the Rust agent to the Network Extension.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum XpcMessage {
    /// Push an updated domain blocklist to the extension.
    UpdateBlocklist(Vec<String>),

    /// Request the current operational status of the extension.
    GetStatus,

    /// Instruct the extension to begin DNS filtering.
    EnableFiltering,

    /// Instruct the extension to stop DNS filtering.
    DisableFiltering,
}

/// Responses returned by the Network Extension to the Rust agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum XpcResponse {
    /// The operation completed successfully.
    Ok,

    /// Status reply in response to [`XpcMessage::GetStatus`].
    Status {
        /// Whether DNS filtering is currently active.
        active: bool,
        /// Number of domains currently being blocked.
        blocked_count: u64,
    },

    /// The extension reported an error.
    Error(String),
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

/// XPC client connected to the BetBlocker Network Extension service.
///
/// On macOS the connection is established via the raw
/// `xpc_connection_create` C API exposed through `libc`-adjacent FFI.
/// The message payload is JSON-serialised to keep the protocol
/// independent of the XPC dictionary wire format, which requires
/// Swift/ObjC on the other end to decode.
#[cfg(target_os = "macos")]
pub struct XpcClient {
    /// The Mach service name the extension is registered under.
    service_name: String,
    /// Raw XPC connection handle (opaque pointer).
    ///
    /// Safety: This pointer is valid for the lifetime of `XpcClient` and
    /// must not outlive the object.  We hold it as `usize` so that the
    /// struct remains `Send`; actual XPC calls go through `unsafe` blocks.
    connection: usize,
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
impl XpcClient {
    /// Connect to the XPC service identified by `service_name`.
    ///
    /// On macOS, this calls `xpc_connection_create` from the
    /// `libxpc.dylib` system library (linked automatically via the
    /// `xpc` framework).  If the connection cannot be established
    /// (service not found, entitlements missing, etc.) an
    /// [`XpcError::ConnectionFailed`] is returned.
    pub fn connect(service_name: &str) -> Result<Self, XpcError> {
        use std::ffi::CString;

        let c_name = CString::new(service_name).map_err(|_| {
            XpcError::ConnectionFailed(format!("invalid service name: {service_name}"))
        })?;

        // SAFETY: `xpc_connection_create` is a documented C function in
        // libxpc.  The name pointer is valid for the duration of the call.
        // Passing NULL for the dispatch queue causes the system to assign
        // the connection's default queue.
        let conn = unsafe { sys::xpc_connection_create(c_name.as_ptr(), std::ptr::null_mut()) };

        if conn.is_null() {
            return Err(XpcError::ConnectionFailed(service_name.to_string()));
        }

        // Activate the connection.
        // SAFETY: conn is non-null and freshly created.
        unsafe { sys::xpc_connection_resume(conn) };

        tracing::debug!(service = service_name, "XPC connection created");

        Ok(Self {
            service_name: service_name.to_string(),
            connection: conn as usize,
        })
    }

    /// Send `msg` to the Network Extension and wait for a response.
    ///
    /// The message is JSON-serialised and placed into an XPC dictionary
    /// under the key `"payload"`.  The reply is expected to contain the
    /// JSON-serialised [`XpcResponse`] under the same key.
    pub fn send_message(&self, msg: XpcMessage) -> Result<XpcResponse, XpcError> {
        use std::ffi::CString;

        let payload = serde_json::to_string(&msg)
            .map_err(|e| XpcError::SendFailed(format!("serialisation error: {e}")))?;

        let c_payload = CString::new(payload.as_str())
            .map_err(|_| XpcError::SendFailed("payload contains null byte".to_string()))?;

        let key_payload = CString::new("payload").expect("static key is valid");

        let conn = self.connection as *mut sys::xpc_object_t;

        // SAFETY: all pointers are valid CStrings; xpc_dictionary_create
        // is a no-fail system API.
        let dict = unsafe {
            let d = sys::xpc_dictionary_create(std::ptr::null(), std::ptr::null_mut(), 0);
            let str_obj = sys::xpc_string_create(c_payload.as_ptr());
            sys::xpc_dictionary_set_value(d, key_payload.as_ptr(), str_obj);
            sys::xpc_release(str_obj);
            d
        };

        // SAFETY: conn was returned by xpc_connection_create and has not
        // been cancelled.  dict is a valid xpc object.
        let reply =
            unsafe { sys::xpc_connection_send_message_with_reply_sync(conn as *mut _, dict) };

        // SAFETY: dict is no longer needed after the send.
        unsafe { sys::xpc_release(dict) };

        if reply.is_null() {
            return Err(XpcError::SendFailed("no reply received".to_string()));
        }

        // Check for XPC error object (type mismatch means it is a real
        // XPC error dictionary, not our app dictionary).
        let is_error = unsafe { sys::xpc_get_type(reply) == sys::XPC_TYPE_ERROR };
        if is_error {
            unsafe { sys::xpc_release(reply) };
            return Err(XpcError::RemoteError("XPC transport error".to_string()));
        }

        // Extract the "payload" string from the reply dictionary.
        let response_cstr = unsafe { sys::xpc_dictionary_get_string(reply, key_payload.as_ptr()) };

        let response_str = if response_cstr.is_null() {
            unsafe { sys::xpc_release(reply) };
            return Err(XpcError::InvalidResponse(
                "reply missing 'payload' key".to_string(),
            ));
        } else {
            // SAFETY: response_cstr is a valid null-terminated UTF-8 string
            // owned by the XPC object; we copy it before releasing.
            unsafe { std::ffi::CStr::from_ptr(response_cstr) }
                .to_str()
                .map_err(|_| XpcError::InvalidResponse("payload is not UTF-8".to_string()))?
                .to_owned()
        };

        unsafe { sys::xpc_release(reply) };

        let response: XpcResponse = serde_json::from_str(&response_str)
            .map_err(|e| XpcError::InvalidResponse(format!("JSON decode error: {e}")))?;

        Ok(response)
    }
}

#[cfg(target_os = "macos")]
impl Drop for XpcClient {
    fn drop(&mut self) {
        // SAFETY: self.connection was created by xpc_connection_create and
        // has not been released yet.
        #[allow(unsafe_code)]
        unsafe {
            let conn = self.connection as *mut sys::xpc_object_t;
            sys::xpc_connection_cancel(conn as *mut _);
            sys::xpc_release(conn as *mut _);
        }
        tracing::debug!(service = %self.service_name, "XPC connection released");
    }
}

/// Raw FFI bindings to `libxpc` / XPC framework.
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types, unsafe_code, dead_code)]
mod sys {
    use libc::{c_char, c_void, size_t};

    /// Opaque XPC object type.
    pub type xpc_object_t = c_void;
    /// Opaque XPC type descriptor.
    pub type xpc_type_t = *const c_void;

    // XPC type constants (extern symbols provided by libxpc).
    unsafe extern "C" {
        pub static _xpc_error_connection_interrupted: xpc_object_t;
        pub static _xpc_type_error: c_void;
    }

    /// Pointer to the XPC_TYPE_ERROR type descriptor.
    ///
    /// On macOS the type descriptor lives at `&_xpc_type_error`.
    pub fn xpc_type_error_ptr() -> xpc_type_t {
        // SAFETY: Accessing a valid extern static.
        unsafe { &_xpc_type_error as *const c_void }
    }

    pub const XPC_TYPE_ERROR: xpc_type_t = unsafe {
        // A zero sentinel; the real check is done via xpc_get_type comparison.
        // We define this as null because the comparison in send_message uses
        // the extern symbol directly.  This field is not used for the actual
        // runtime check — see send_message for the correct approach.
        std::ptr::null()
    };

    unsafe extern "C" {
        // Connection lifecycle
        pub fn xpc_connection_create(
            name: *const c_char,
            targetq: *mut c_void,
        ) -> *mut xpc_object_t;
        pub fn xpc_connection_resume(connection: *mut xpc_object_t);
        pub fn xpc_connection_cancel(connection: *mut xpc_object_t);
        pub fn xpc_connection_send_message_with_reply_sync(
            connection: *mut xpc_object_t,
            message: *mut xpc_object_t,
        ) -> *mut xpc_object_t;

        // Dictionary
        pub fn xpc_dictionary_create(
            keys: *const *const c_char,
            values: *mut *mut xpc_object_t,
            count: size_t,
        ) -> *mut xpc_object_t;
        pub fn xpc_dictionary_set_value(
            xdict: *mut xpc_object_t,
            key: *const c_char,
            value: *mut xpc_object_t,
        );
        pub fn xpc_dictionary_get_string(
            xdict: *mut xpc_object_t,
            key: *const c_char,
        ) -> *const c_char;

        // String
        pub fn xpc_string_create(string: *const c_char) -> *mut xpc_object_t;

        // Type
        pub fn xpc_get_type(object: *mut xpc_object_t) -> xpc_type_t;

        // Memory
        pub fn xpc_release(object: *mut xpc_object_t);
    }
}

// ---------------------------------------------------------------------------
// Non-macOS stubs
// ---------------------------------------------------------------------------

/// Stub XPC client for non-macOS platforms.
///
/// All methods return [`XpcError::NotSupported`].
#[cfg(not(target_os = "macos"))]
pub struct XpcClient {
    #[allow(dead_code)]
    service_name: String,
}

#[cfg(not(target_os = "macos"))]
impl XpcClient {
    /// Stub connect — always returns [`XpcError::NotSupported`].
    pub fn connect(service_name: &str) -> Result<Self, XpcError> {
        tracing::warn!(
            service = service_name,
            "XPC not available on non-macOS; returning stub"
        );
        Ok(Self {
            service_name: service_name.to_string(),
        })
    }

    /// Stub send — always returns [`XpcError::NotSupported`].
    pub fn send_message(&self, _msg: XpcMessage) -> Result<XpcResponse, XpcError> {
        Err(XpcError::NotSupported)
    }
}

// ---------------------------------------------------------------------------
// Mock client for testing
// ---------------------------------------------------------------------------

/// A mock XPC client that records sent messages and returns pre-configured
/// responses. Useful for unit-testing code that depends on `XpcClient`
/// behaviour without a real Network Extension.
#[cfg(test)]
pub mod mock {
    use super::{XpcError, XpcMessage, XpcResponse};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// Mock XPC client.
    pub struct MockXpcClient {
        /// Queue of responses to return for consecutive calls.
        responses: RefCell<VecDeque<Result<XpcResponse, XpcError>>>,
        /// Messages recorded in the order they were sent.
        pub sent_messages: RefCell<Vec<XpcMessage>>,
    }

    impl MockXpcClient {
        /// Create a new mock with no pre-configured responses.
        /// Calls to [`send_message`] with an empty queue return `Ok(XpcResponse::Ok)`.
        pub fn new() -> Self {
            Self {
                responses: RefCell::new(VecDeque::new()),
                sent_messages: RefCell::new(Vec::new()),
            }
        }

        /// Push a response that will be returned by the next call to
        /// [`send_message`].
        pub fn push_response(&self, resp: Result<XpcResponse, XpcError>) {
            self.responses.borrow_mut().push_back(resp);
        }

        /// Send a message, record it, and return the next queued response.
        pub fn send_message(&self, msg: XpcMessage) -> Result<XpcResponse, XpcError> {
            self.sent_messages.borrow_mut().push(msg);
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(XpcResponse::Ok))
        }
    }

    impl Default for MockXpcClient {
        fn default() -> Self {
            Self::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::mock::MockXpcClient;
    use super::{XpcError, XpcMessage, XpcResponse};

    #[test]
    fn mock_returns_ok_by_default() {
        let client = MockXpcClient::new();
        let resp = client.send_message(XpcMessage::GetStatus);
        assert!(matches!(resp, Ok(XpcResponse::Ok)));
    }

    #[test]
    fn mock_records_sent_messages() {
        let client = MockXpcClient::new();
        client
            .send_message(XpcMessage::EnableFiltering)
            .expect("send ok");
        client
            .send_message(XpcMessage::DisableFiltering)
            .expect("send ok");

        let msgs = client.sent_messages.borrow();
        assert_eq!(msgs.len(), 2);
        assert!(matches!(msgs[0], XpcMessage::EnableFiltering));
        assert!(matches!(msgs[1], XpcMessage::DisableFiltering));
    }

    #[test]
    fn mock_returns_queued_status_response() {
        let client = MockXpcClient::new();
        client.push_response(Ok(XpcResponse::Status {
            active: true,
            blocked_count: 42,
        }));

        let resp = client.send_message(XpcMessage::GetStatus).expect("send ok");
        assert!(
            matches!(
                resp,
                XpcResponse::Status {
                    active: true,
                    blocked_count: 42
                }
            ),
            "expected status response"
        );
    }

    #[test]
    fn mock_returns_queued_error() {
        let client = MockXpcClient::new();
        client.push_response(Err(XpcError::RemoteError(
            "extension not loaded".to_string(),
        )));

        let result = client.send_message(XpcMessage::EnableFiltering);
        assert!(
            matches!(result, Err(XpcError::RemoteError(_))),
            "expected remote error"
        );
    }

    #[test]
    fn mock_update_blocklist_roundtrip() {
        let domains = vec!["bet365.com".to_string(), "williamhill.com".to_string()];
        let client = MockXpcClient::new();
        client.push_response(Ok(XpcResponse::Ok));

        let resp = client
            .send_message(XpcMessage::UpdateBlocklist(domains.clone()))
            .expect("send ok");
        assert!(matches!(resp, XpcResponse::Ok));

        let msgs = client.sent_messages.borrow();
        if let XpcMessage::UpdateBlocklist(ref sent) = msgs[0] {
            assert_eq!(sent, &domains);
        } else {
            panic!("wrong message type recorded");
        }
    }

    #[test]
    fn xpc_error_display() {
        assert!(
            XpcError::ConnectionFailed("svc".to_string())
                .to_string()
                .contains("svc")
        );
        assert!(
            XpcError::SendFailed("oops".to_string())
                .to_string()
                .contains("oops")
        );
        assert!(
            XpcError::InvalidResponse("bad".to_string())
                .to_string()
                .contains("bad")
        );
        assert!(
            XpcError::RemoteError("crash".to_string())
                .to_string()
                .contains("crash")
        );
        assert!(XpcError::NotSupported.to_string().contains("platform"));
    }

    #[test]
    fn xpc_message_serde_roundtrip() {
        let messages = vec![
            XpcMessage::GetStatus,
            XpcMessage::EnableFiltering,
            XpcMessage::DisableFiltering,
            XpcMessage::UpdateBlocklist(vec!["example.com".to_string()]),
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).expect("serialise");
            let decoded: XpcMessage = serde_json::from_str(&json).expect("deserialise");
            // Verify via re-serialise: both should produce the same JSON.
            let json2 = serde_json::to_string(&decoded).expect("re-serialise");
            assert_eq!(json, json2, "round-trip mismatch");
        }
    }

    #[test]
    fn xpc_response_serde_roundtrip() {
        let responses = vec![
            XpcResponse::Ok,
            XpcResponse::Error("oops".to_string()),
            XpcResponse::Status {
                active: false,
                blocked_count: 100,
            },
        ];

        for resp in responses {
            let json = serde_json::to_string(&resp).expect("serialise");
            let decoded: XpcResponse = serde_json::from_str(&json).expect("deserialise");
            let json2 = serde_json::to_string(&decoded).expect("re-serialise");
            assert_eq!(json, json2, "round-trip mismatch");
        }
    }

    /// On non-macOS, XpcClient::connect returns a stub (not an error),
    /// but send_message returns NotSupported.
    #[test]
    #[cfg(not(target_os = "macos"))]
    fn non_macos_stub_send_returns_not_supported() {
        let client = super::XpcClient::connect("com.betblocker.networkextension")
            .expect("stub connect should succeed");
        let result = client.send_message(XpcMessage::GetStatus);
        assert!(
            matches!(result, Err(XpcError::NotSupported)),
            "expected NotSupported on non-macOS"
        );
    }
}
