use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;

use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::services::tor_exits_service;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /v1/tor-exits
// ---------------------------------------------------------------------------

/// Return the current Tor exit node IP list as JSON.
///
/// The list is read from the `tor_exit_nodes` database table, which is kept
/// fresh by `TorExitNodeRefreshJob` in `bb-worker`.
pub async fn get_tor_exits(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let nodes = tor_exits_service::list_exit_nodes(&state.db).await?;

    Ok(ApiResponse::ok(json!({
        "nodes": nodes,
        "count": nodes.len(),
    })))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Integration-level tests require a live DB, so we just verify the module
    /// compiles and exports the handler.
    #[test]
    fn handler_is_accessible() {
        // If this compiles, the function is exported correctly.
        let _ = std::any::type_name_of_val(&super::get_tor_exits);
    }
}
