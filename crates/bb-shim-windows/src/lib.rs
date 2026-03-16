// Platform shim: many functions are cross-platform stubs.
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::items_after_statements,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_self,
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::too_many_lines
)]

//! Windows platform shim for `BetBlocker`.
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
