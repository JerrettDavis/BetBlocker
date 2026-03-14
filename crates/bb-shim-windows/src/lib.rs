//! Windows platform shim for BetBlocker.
//!
//! Provides Windows-specific implementations for service management,
//! DNS monitoring, ACL enforcement, keystore, installer, and updater.

pub mod acl;
pub mod dns_monitor;
pub mod installer;
pub mod keystore;
pub mod service;
pub mod updater;

#[cfg(feature = "kernel-drivers")]
pub mod minifilter;
#[cfg(feature = "kernel-drivers")]
pub mod wfp;
