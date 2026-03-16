mod common;

use serde_json::{Value, json};

/// Test device registration.
#[tokio::test]
async fn test_device_registration() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app.register_user("dev@example.com", "SecurePass123!").await;

    let resp = app
        .authed_post(
            "/v1/devices",
            &token,
            &json!({
                "name": "Test MacBook",
                "platform": "macos",
                "os_version": "15.3.1",
                "agent_version": "1.0.0",
                "hostname": "test-mbp.local",
                "hardware_id": "hw_abc123"
            }),
        )
        .await;

    assert_eq!(resp.status().as_u16(), 201);
    let body: Value = resp.json().await.unwrap();
    assert!(body["data"]["device"]["id"].is_string());
    assert!(body["data"]["device_token"].is_string());
    let device_token = body["data"]["device_token"].as_str().unwrap();
    assert!(device_token.starts_with("dtk_"));
}

/// Test duplicate hardware_id returns 409.
#[tokio::test]
async fn test_device_duplicate_hardware_id() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app
        .register_user("dup_hw@example.com", "SecurePass123!")
        .await;

    let device_json = json!({
        "name": "Device 1",
        "platform": "windows",
        "os_version": "11",
        "agent_version": "1.0.0",
        "hostname": "desktop",
        "hardware_id": "hw_duplicate"
    });

    let resp = app.authed_post("/v1/devices", &token, &device_json).await;
    assert_eq!(resp.status().as_u16(), 201);

    // Second registration with same hardware_id
    let resp = app
        .authed_post(
            "/v1/devices",
            &token,
            &json!({
                "name": "Device 2",
                "platform": "windows",
                "os_version": "11",
                "agent_version": "1.0.0",
                "hostname": "desktop2",
                "hardware_id": "hw_duplicate"
            }),
        )
        .await;

    assert_eq!(resp.status().as_u16(), 409);
}

/// Test device listing.
#[tokio::test]
async fn test_device_listing() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app
        .register_user("list@example.com", "SecurePass123!")
        .await;

    // Register 3 devices
    for i in 0..3 {
        app.authed_post(
            "/v1/devices",
            &token,
            &json!({
                "name": format!("Device {i}"),
                "platform": "linux",
                "os_version": "6.5",
                "agent_version": "1.0.0",
                "hostname": format!("host{i}"),
                "hardware_id": format!("hw_{i}")
            }),
        )
        .await;
    }

    let resp = app.authed_get("/v1/devices", &token).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    assert_eq!(body["pagination"]["total"], 3);
}

/// Test heartbeat updates device.
#[tokio::test]
async fn test_heartbeat() {
    let app = common::TestApp::spawn().await;

    let (_id, token, _rt) = app.register_user("hb@example.com", "SecurePass123!").await;

    // Register device
    let resp = app
        .authed_post(
            "/v1/devices",
            &token,
            &json!({
                "name": "Heartbeat Device",
                "platform": "macos",
                "os_version": "15.0",
                "agent_version": "1.0.0",
                "hostname": "hb-mac",
                "hardware_id": "hw_hb"
            }),
        )
        .await;

    let body: Value = resp.json().await.unwrap();
    let device_id = body["data"]["device"]["id"].as_str().unwrap();

    // Send heartbeat
    let resp = app
        .authed_post(
            &format!("/v1/devices/{device_id}/heartbeat"),
            &token,
            &json!({
                "agent_version": "1.1.0",
                "os_version": "15.1",
                "blocklist_version": 100,
                "uptime_seconds": 3600,
                "blocking_active": true
            }),
        )
        .await;

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["ack"], true);
    assert!(body["data"]["next_heartbeat_seconds"].is_number());
}
