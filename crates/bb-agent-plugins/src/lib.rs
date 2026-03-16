// Clippy pedantic: allow these at crate level for now
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_fields_in_debug)]
#![allow(clippy::unnecessary_literal_bound)]

//! `BetBlocker` Agent Plugins -- blocking plugin trait definitions and built-in plugins.
//!
//! This crate defines the plugin trait hierarchy (`BlockingPlugin`, `DnsBlockingPlugin`, etc.),
//! supporting types, the blocklist engine, and the built-in DNS resolver and HOSTS file plugins.

#[cfg(feature = "app-process")]
pub mod app_process;
pub mod blocklist;
pub mod registry;
pub mod traits;
pub mod types;

#[cfg(feature = "dns-resolver")]
pub mod dns_resolver;

#[cfg(feature = "dns-hosts")]
pub mod hosts_file;

// Re-exports for convenience
pub use blocklist::Blocklist;
pub use registry::{PluginInstance, PluginRegistry};
pub use traits::{AppBlockingPlugin, BlockingPlugin, ContentBlockingPlugin, DnsBlockingPlugin};
pub use types::{
    AppIdentifier, AppMatch, AppMatchType, BlockDecision, BlockingLayer, ContentRules,
    ExtensionHealth, PluginConfig, PluginError, PluginHealth,
};
