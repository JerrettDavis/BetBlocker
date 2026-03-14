//! macOS platform shim for BetBlocker.
//!
//! Provides macOS-specific implementations for launchd service management,
//! Keychain integration, DNS monitoring, Network Extension, and file protection.

pub mod dns_monitor;
pub mod file_protect;
pub mod installer;
pub mod keychain;
pub mod launchd;
pub mod network_ext;
pub mod platform;
pub mod xpc;
