// Pedantic clippy: allow common lints at crate level.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unused_async,
    clippy::unused_self,
    clippy::single_match_else,
    clippy::match_same_arms,
    clippy::manual_let_else,
    clippy::needless_borrows_for_generic_args,
    clippy::uninlined_format_args,
    clippy::wildcard_imports,
    clippy::default_constructed_unit_structs,
    clippy::return_self_not_must_use,
    clippy::needless_pass_by_value,
    clippy::unwrap_used
)]

pub mod config;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod response;
pub mod routes;
pub mod services;
pub mod state;
