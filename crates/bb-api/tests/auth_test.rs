mod common;

use serde_json::{Value, json};

/// Test user registration.
#[tokio::test]
async fn test_register_user() {
    let app = common::TestApp::spawn().await;

    let (account_id, access_token, refresh_token) = app
        .register_user("test@example.com", "SecurePass123!")
        .await;

    assert!(!account_id.is_empty());
    assert!(!access_token.is_empty());
    assert!(refresh_token.starts_with("rtk_"));
}

/// Test duplicate email returns 409.
#[tokio::test]
async fn test_register_duplicate_email() {
    let app = common::TestApp::spawn().await;

    app.register_user("dup@example.com", "SecurePass123!").await;

    // Second registration with same email
    let resp = app
        .client
        .post(format!("{}/v1/auth/register", app.address))
        .json(&json!({
            "email": "dup@example.com",
            "password": "SecurePass123!",
            "display_name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "EMAIL_ALREADY_EXISTS");
}

/// Test login with valid credentials.
#[tokio::test]
async fn test_login_success() {
    let app = common::TestApp::spawn().await;

    app.register_user("login@example.com", "SecurePass123!")
        .await;
    let (access_token, refresh_token) = app.login("login@example.com", "SecurePass123!").await;

    assert!(!access_token.is_empty());
    assert!(refresh_token.starts_with("rtk_"));
}

/// Test login with wrong password returns generic 401.
#[tokio::test]
async fn test_login_wrong_password() {
    let app = common::TestApp::spawn().await;

    app.register_user("wrong@example.com", "SecurePass123!")
        .await;

    let resp = app
        .client
        .post(format!("{}/v1/auth/login", app.address))
        .json(&json!({
            "email": "wrong@example.com",
            "password": "WrongPassword1!"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "INVALID_CREDENTIALS");
}

/// Test login with nonexistent email returns same 401 (no enumeration).
#[tokio::test]
async fn test_login_nonexistent_email() {
    let app = common::TestApp::spawn().await;

    let resp = app
        .client
        .post(format!("{}/v1/auth/login", app.address))
        .json(&json!({
            "email": "nobody@example.com",
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "INVALID_CREDENTIALS");
}

/// Test refresh token rotation.
#[tokio::test]
async fn test_refresh_token_rotation() {
    let app = common::TestApp::spawn().await;

    let (_id, _at, refresh1) = app
        .register_user("refresh@example.com", "SecurePass123!")
        .await;

    // Use refresh token to get new tokens
    let resp = app
        .client
        .post(format!("{}/v1/auth/refresh", app.address))
        .json(&json!({ "refresh_token": refresh1 }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    let refresh2 = body["data"]["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(refresh1, refresh2);

    // Old refresh token should now be rejected
    let resp = app
        .client
        .post(format!("{}/v1/auth/refresh", app.address))
        .json(&json!({ "refresh_token": refresh1 }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
    let body: Value = resp.json().await.unwrap();
    // Reuse detection: old token was marked as used, so using it again triggers family revocation
    assert_eq!(body["error"]["code"], "TOKEN_FAMILY_REVOKED");
}

/// Test logout revokes refresh token.
#[tokio::test]
async fn test_logout() {
    let app = common::TestApp::spawn().await;

    let (_id, access_token, refresh_token) = app
        .register_user("logout@example.com", "SecurePass123!")
        .await;

    // Logout
    let resp = app
        .authed_post(
            "/v1/auth/logout",
            &access_token,
            &json!({ "refresh_token": refresh_token }),
        )
        .await;

    assert_eq!(resp.status().as_u16(), 204);

    // Try to use revoked refresh token
    let resp = app
        .client
        .post(format!("{}/v1/auth/refresh", app.address))
        .json(&json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
}

/// Test forgot-password always returns 202.
#[tokio::test]
async fn test_forgot_password() {
    let app = common::TestApp::spawn().await;

    // Existing user
    app.register_user("forgot@example.com", "SecurePass123!")
        .await;

    let resp = app
        .client
        .post(format!("{}/v1/auth/forgot-password", app.address))
        .json(&json!({ "email": "forgot@example.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 202);

    // Non-existing user also returns 202 (prevent enumeration)
    let resp = app
        .client
        .post(format!("{}/v1/auth/forgot-password", app.address))
        .json(&json!({ "email": "noone@example.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 202);
}
