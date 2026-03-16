// Platform shim: many functions are cross-platform stubs.
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::if_not_else,
    clippy::collapsible_if
)]

//! Linux platform shim for `BetBlocker`.
//!
//! Provides Linux-specific implementations for Mandatory Access Control
//! (AppArmor, SELinux) and optional eBPF-based network filtering.

pub mod apparmor;
pub mod mac;
pub mod selinux;

#[cfg(feature = "ebpf")]
pub mod ebpf;
