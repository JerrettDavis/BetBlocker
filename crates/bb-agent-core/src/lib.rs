// Clippy pedantic: allow these at crate level for now
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::doc_markdown)]

//! `BetBlocker` Agent Core -- cross-platform blocking engine.
//!
//! This crate contains the platform-independent blocking logic:
//! event system, configuration management, API communication,
//! tamper resistance, and orchestration.
//! Plugin traits, blocklist engine, and built-in plugins live in `bb-agent-plugins`.

pub mod bypass_detection;
pub mod config;
pub mod events;
pub mod comms;
pub mod tamper;

// Re-exports from bb-agent-plugins for convenience
pub use bb_agent_plugins::blocklist;
pub use bb_agent_plugins::registry;
pub use bb_agent_plugins::traits;
pub use bb_agent_plugins::types;
