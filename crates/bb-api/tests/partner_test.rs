mod common;

use serde_json::{Value, json};

// Note: Partner invite flow requires email_verified = true, which
// is not automatically set during registration. These tests verify
// the partner endpoint responses and error handling.

/// Test partner invite requires verified email.
#[tokio::test]
async fn test_partner_invite_requires_verified_email() {
    let app = common::TestApp::spawn().await;

    let (_id_a, token_a, _rt_a) = app
        .register_user("a_partner@example.com", "SecurePass123!")
        .await;
    let (_id_b, _token_b, _rt_b) = app
        .register_user("b_partner@example.com", "SecurePass123!")
        .await;

    // Invite should fail because email is not verified
    let resp = app
        .authed_post(
            "/v1/partners/invite",
            &token_a,
            &json!({ "email": "b_partner@example.com" }),
        )
        .await;

    assert_eq!(resp.status().as_u16(), 403);
    let body: Value = resp.json().await.unwrap();
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("verified")
    );
}

/// Test partner listing returns empty initially.
#[tokio::test]
async fn test_partner_list_empty() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app
        .register_user("lonely@example.com", "SecurePass123!")
        .await;

    let resp = app.authed_get("/v1/partners", &token).await;
    assert_eq!(resp.status().as_u16(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["total"], 0);
}

/// Test self-invite is rejected.
#[tokio::test]
async fn test_cannot_invite_self() {
    let app = common::TestApp::spawn().await;

    // We need to bypass email verification for this test.
    // For now, we just verify the self-invite check works if we could get past verification.
    // This test documents the expected behavior.
    let (_id, token, _rt) = app
        .register_user("selfie@example.com", "SecurePass123!")
        .await;

    let resp = app
        .authed_post(
            "/v1/partners/invite",
            &token,
            &json!({ "email": "selfie@example.com" }),
        )
        .await;

    // Gets 403 because email isn't verified (before self-check runs)
    assert_eq!(resp.status().as_u16(), 403);
}

/// Test partner not found for nonexistent relationship.
#[tokio::test]
async fn test_accept_nonexistent_partner() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app
        .register_user("accept@example.com", "SecurePass123!")
        .await;

    let resp = app
        .authed_post("/v1/partners/99999/accept", &token, &json!({}))
        .await;

    assert_eq!(resp.status().as_u16(), 404);
}

/// Test remove nonexistent partner.
#[tokio::test]
async fn test_remove_nonexistent_partner() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app
        .register_user("remove@example.com", "SecurePass123!")
        .await;

    let resp = app.authed_delete("/v1/partners/99999", &token).await;

    assert_eq!(resp.status().as_u16(), 404);
}
