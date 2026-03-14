pub mod admin_blocklist;
pub mod accounts;
pub mod auth;
pub mod billing;
pub mod blocklist;
pub mod devices;
pub mod enrollments;
pub mod events;
pub mod health;
pub mod organizations;
pub mod partners;

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::middleware::request_id::UuidV7RequestId;
use crate::state::AppState;

/// Build the complete Axum router with all route groups and middleware.
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any) // TODO: restrict to config.cors_allowed_origins in production
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([axum::http::HeaderName::from_static("x-request-id")]);

    let x_request_id = axum::http::HeaderName::from_static("x-request-id");

    // Auth routes (unauthenticated)
    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh))
        .route("/logout", post(auth::logout))
        .route("/forgot-password", post(auth::forgot_password))
        .route("/reset-password", post(auth::reset_password));

    // Account routes (authenticated)
    let account_routes = Router::new()
        .route("/me", get(accounts::get_me).patch(accounts::update_me))
        .route("/{id}", get(accounts::get_account));

    // Device routes (authenticated)
    let device_routes = Router::new()
        .route("/", post(devices::register_device).get(devices::list_devices))
        .route(
            "/{id}",
            get(devices::get_device).delete(devices::delete_device),
        )
        .route("/{id}/heartbeat", post(devices::heartbeat))
        .route("/{id}/config", get(devices::get_device_config));

    // Enrollment routes (authenticated)
    let enrollment_routes = Router::new()
        .route(
            "/",
            post(enrollments::create_enrollment).get(enrollments::list_enrollments),
        )
        .route(
            "/{id}",
            get(enrollments::get_enrollment).patch(enrollments::update_enrollment),
        )
        .route("/{id}/unenroll", post(enrollments::request_unenroll))
        .route(
            "/{id}/approve-unenroll",
            post(enrollments::approve_unenroll),
        );

    // Organization routes (authenticated)
    let organization_routes = Router::new()
        .route("/", post(organizations::create_org).get(organizations::list_orgs))
        .route(
            "/{id}",
            get(organizations::get_org)
                .patch(organizations::update_org)
                .delete(organizations::delete_org),
        )
        .route(
            "/{id}/members",
            post(organizations::invite_member).get(organizations::list_members),
        )
        .route(
            "/{id}/members/{member_id}",
            patch(organizations::update_member_role)
                .delete(organizations::remove_member),
        );

    // Partner routes (authenticated)
    let partner_routes = Router::new()
        .route("/invite", post(partners::invite_partner))
        .route("/", get(partners::list_partners))
        .route("/{id}/accept", post(partners::accept_partner))
        .route("/{id}", delete(partners::remove_partner));

    // Blocklist routes (public version/delta, authenticated report)
    let blocklist_routes = Router::new()
        .route("/version", get(blocklist::get_version))
        .route("/delta", get(blocklist::get_delta))
        .route("/report", post(blocklist::submit_report));

    // Admin blocklist routes (admin only)
    let admin_blocklist_routes = Router::new()
        .route(
            "/entries",
            post(admin_blocklist::create_entry).get(admin_blocklist::list_entries),
        )
        .route(
            "/entries/{id}",
            patch(admin_blocklist::update_entry).delete(admin_blocklist::delete_entry),
        )
        .route("/review-queue", get(admin_blocklist::review_queue))
        .route(
            "/review-queue/{domain}/resolve",
            post(admin_blocklist::resolve_review),
        );

    // Event routes (authenticated)
    let event_routes = Router::new()
        .route("/", post(events::batch_ingest).get(events::query_events))
        .route("/summary", get(events::event_summary));

    // Assemble the API under /v1
    let mut api = Router::new()
        .nest("/v1/auth", auth_routes)
        .nest("/v1/accounts", account_routes)
        .nest("/v1/devices", device_routes)
        .nest("/v1/enrollments", enrollment_routes)
        .nest("/v1/organizations", organization_routes)
        .nest("/v1/partners", partner_routes)
        .nest("/v1/blocklist", blocklist_routes)
        .nest("/v1/admin/blocklist", admin_blocklist_routes)
        .nest("/v1/events", event_routes);

    // Conditionally register billing routes
    if state.config.billing_enabled {
        let billing_routes = Router::new()
            .route("/subscribe", post(billing::subscribe))
            .route("/status", get(billing::billing_status))
            .route("/webhook", post(billing::webhook))
            .route("/cancel", post(billing::cancel));
        api = api.nest("/v1/billing", billing_routes);
    }

    // Health check at root
    api = api.route("/health", get(health::health_check));

    // Apply middleware stack
    api.layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(
            x_request_id,
            UuidV7RequestId::default(),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
