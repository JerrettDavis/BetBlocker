// Pedantic clippy: allow common lints at crate level.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::single_match_else,
    clippy::match_same_arms,
    clippy::unused_self,
    clippy::used_underscore_binding,
    clippy::ignored_unit_patterns,
    clippy::format_collect,
    clippy::return_self_not_must_use,
    clippy::needless_pass_by_value,
    clippy::expect_used
)]

//! `BetBlocker` Agent Core -- cross-platform blocking engine.
//!
//! This crate contains the platform-independent blocking logic:
//! event system, configuration management, API communication,
//! tamper resistance, and orchestration.
//! Plugin traits, blocklist engine, and built-in plugins live in `bb-agent-plugins`.

#[cfg(feature = "bypass-detection")]
pub mod bypass_detection;
pub mod comms;
pub mod config;
pub mod events;
pub mod federated;
pub mod tamper;

// Re-exports from bb-agent-plugins for convenience
pub use bb_agent_plugins::blocklist;
pub use bb_agent_plugins::registry;
pub use bb_agent_plugins::traits;
pub use bb_agent_plugins::types;
