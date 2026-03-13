use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<&'static str>,
}

/// GET /health -- returns server status, optionally checking DB and Redis.
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let redis_ok = match state.redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .is_ok(),
        Err(_) => false,
    };

    let status = if db_ok && redis_ok {
        "ok"
    } else {
        "degraded"
    };

    (
        StatusCode::OK,
        Json(HealthResponse {
            status,
            version: env!("CARGO_PKG_VERSION"),
            database: Some(if db_ok { "ok" } else { "unreachable" }),
            redis: Some(if redis_ok { "ok" } else { "unreachable" }),
        }),
    )
}
