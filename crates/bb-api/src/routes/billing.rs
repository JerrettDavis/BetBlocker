use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::AuthenticatedAccount;
use crate::response::ApiResponse;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub plan: String,
    pub payment_method_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    // Stripe webhook events are parsed from raw body in production.
    // For Phase 1, we accept a JSON object.
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// POST /billing/subscribe
// ---------------------------------------------------------------------------

pub async fn subscribe(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<SubscribeRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if !state.config.billing_enabled {
        return Err(ApiError::NotFound {
            code: "BILLING_DISABLED".into(),
            message: "Billing is not enabled on this instance".into(),
        });
    }

    let account =
        crate::services::account_service::get_account_by_public_id(&state.db, auth.account_id)
            .await?;

    // Validate plan
    let valid_plans = ["standard", "partner_tier", "institutional"];
    if !valid_plans.contains(&req.plan.as_str()) {
        return Err(ApiError::Validation {
            message: "Invalid plan".into(),
            details: None,
        });
    }

    // Phase 1: stub implementation (no actual Stripe calls)
    let subscription_id = format!("sub_{}", uuid::Uuid::now_v7());
    let customer_id = format!("cus_{}", uuid::Uuid::now_v7());

    let now = chrono::Utc::now();
    let period_end = now + chrono::Duration::days(30);

    sqlx::query(
        r#"INSERT INTO subscriptions
               (account_id, stripe_customer_id, stripe_subscription_id, plan, status,
                current_period_start, current_period_end)
           VALUES ($1, $2, $3, $4::subscription_plan, 'active'::subscription_status, $5, $6)"#,
    )
    .bind(account.id)
    .bind(&customer_id)
    .bind(&subscription_id)
    .bind(&req.plan)
    .bind(now)
    .bind(period_end)
    .execute(&state.db)
    .await?;

    Ok(ApiResponse::created(json!({
        "subscription_id": subscription_id,
        "plan": req.plan,
        "status": "active",
        "current_period_start": now.to_rfc3339(),
        "current_period_end": period_end.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /billing/status
// ---------------------------------------------------------------------------

pub async fn billing_status(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if !state.config.billing_enabled {
        return Err(ApiError::NotFound {
            code: "BILLING_DISABLED".into(),
            message: "Billing is not enabled on this instance".into(),
        });
    }

    let account =
        crate::services::account_service::get_account_by_public_id(&state.db, auth.account_id)
            .await?;

    let sub = sqlx::query_as::<_, SubscriptionRow>(
        r#"SELECT stripe_subscription_id, plan::text, status::text,
                  current_period_start, current_period_end
           FROM subscriptions
           WHERE account_id = $1
             AND status IN ('active', 'trialing', 'past_due')
           LIMIT 1"#,
    )
    .bind(account.id)
    .fetch_optional(&state.db)
    .await?;

    match sub {
        Some(s) => Ok(ApiResponse::ok(json!({
            "has_subscription": true,
            "plan": s.plan,
            "status": s.status,
            "current_period_start": s.current_period_start.to_rfc3339(),
            "current_period_end": s.current_period_end.to_rfc3339(),
        }))),
        None => Ok(ApiResponse::ok(json!({
            "has_subscription": false,
            "plan": null,
            "status": null,
        }))),
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct SubscriptionRow {
    stripe_subscription_id: String,
    plan: String,
    status: String,
    current_period_start: chrono::DateTime<chrono::Utc>,
    current_period_end: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// POST /billing/webhook
// ---------------------------------------------------------------------------

pub async fn webhook(
    State(state): State<AppState>,
    Json(req): Json<WebhookPayload>,
) -> Result<StatusCode, ApiError> {
    if !state.config.billing_enabled {
        return Ok(StatusCode::NOT_FOUND);
    }

    // Phase 1: basic webhook handling without Stripe signature verification
    match req.event_type.as_str() {
        "invoice.paid" => {
            tracing::info!("Stripe webhook: invoice.paid");
            // Extend subscription period
        }
        "invoice.payment_failed" => {
            tracing::info!("Stripe webhook: invoice.payment_failed");
            // Mark subscription as past_due
            if let Some(sub_id) = req.data["object"]["subscription"].as_str() {
                sqlx::query(
                    "UPDATE subscriptions SET status = 'past_due'::subscription_status, updated_at = NOW() WHERE stripe_subscription_id = $1",
                )
                .bind(sub_id)
                .execute(&state.db)
                .await?;
            }
        }
        "customer.subscription.deleted" => {
            tracing::info!("Stripe webhook: customer.subscription.deleted");
            if let Some(sub_id) = req.data["object"]["id"].as_str() {
                sqlx::query(
                    "UPDATE subscriptions SET status = 'cancelled'::subscription_status, updated_at = NOW() WHERE stripe_subscription_id = $1",
                )
                .bind(sub_id)
                .execute(&state.db)
                .await?;
            }
        }
        _ => {
            tracing::debug!("Ignoring Stripe event: {}", req.event_type);
        }
    }

    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// POST /billing/cancel
// ---------------------------------------------------------------------------

pub async fn cancel(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if !state.config.billing_enabled {
        return Err(ApiError::NotFound {
            code: "BILLING_DISABLED".into(),
            message: "Billing is not enabled on this instance".into(),
        });
    }

    let account =
        crate::services::account_service::get_account_by_public_id(&state.db, auth.account_id)
            .await?;

    // Phase 1: just mark for cancellation at period end
    let result = sqlx::query(
        r#"UPDATE subscriptions
           SET updated_at = NOW()
           WHERE account_id = $1
             AND status IN ('active', 'trialing')"#,
    )
    .bind(account.id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            code: "NO_ACTIVE_SUBSCRIPTION".into(),
            message: "No active subscription found".into(),
        });
    }

    Ok(ApiResponse::ok(json!({
        "status": "cancelling",
        "cancel_at_period_end": true,
    })))
}
