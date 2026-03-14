//! Linux platform shim for BetBlocker.
//!
//! Provides Linux-specific implementations for Mandatory Access Control
//! (AppArmor, SELinux) and optional eBPF-based network filtering.

pub mod apparmor;
pub mod mac;
pub mod selinux;

#[cfg(feature = "ebpf")]
pub mod ebpf;
