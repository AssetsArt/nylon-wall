mod common;

use common::{TestServer, sample_rule, sample_rule_with_name, create_and_confirm};

#[tokio::test]
async fn test_no_pending_initially() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/changes/pending")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let status: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status["pending"], false);
}

#[tokio::test]
async fn test_create_creates_pending() {
    let server = TestServer::start().await;

    // Create a rule (this creates a pending change)
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);

    // Check pending
    let resp = server.client.get(server.url("/api/v1/changes/pending")).send().await.unwrap();
    let status: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status["pending"], true);
    assert!(status["remaining_secs"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_confirm_clears_pending() {
    let server = TestServer::start().await;

    // Create rule (pending)
    server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();

    // Confirm
    server.confirm().await;

    // Check no longer pending
    let resp = server.client.get(server.url("/api/v1/changes/pending")).send().await.unwrap();
    let status: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status["pending"], false);
}

#[tokio::test]
async fn test_revert_undoes_create() {
    let server = TestServer::start().await;

    // Create rule
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();

    // Revert
    let resp = server.client.post(server.url("/api/v1/changes/revert")).send().await.unwrap();
    assert!(resp.status().is_success());

    // Rule should be gone
    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_revert_undoes_update() {
    let server = TestServer::start().await;

    // Create and confirm
    let created = create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    let id = created["id"].as_u64().unwrap();

    // Update (creates new pending change)
    let mut updated = created.clone();
    updated["name"] = serde_json::json!("Modified Name");
    server.client.put(server.url(&format!("/api/v1/rules/{}", id)))
        .json(&updated)
        .send().await.unwrap();

    // Revert the update
    server.client.post(server.url("/api/v1/changes/revert")).send().await.unwrap();

    // Should still have original name
    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    let rule: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(rule["name"], "Test Rule");
}

#[tokio::test]
async fn test_revert_undoes_delete() {
    let server = TestServer::start().await;

    // Create and confirm
    let created = create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    let id = created["id"].as_u64().unwrap();

    // Delete (creates pending change)
    server.client.delete(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();

    // Rule should be gone
    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);

    // Revert delete
    server.client.post(server.url("/api/v1/changes/revert")).send().await.unwrap();

    // Rule should be back
    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let rule: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(rule["name"], "Test Rule");
}

#[tokio::test]
async fn test_concurrent_change_blocked() {
    let server = TestServer::start().await;

    // First create succeeds
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule_with_name("Rule A"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);

    // Second create while first is pending -> 409
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule_with_name("Rule B"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 409);

    // Confirm first, then second succeeds
    server.confirm().await;
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule_with_name("Rule B"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
}

// Note: auto_revert test is in its own file (api_auto_revert.rs)
// to avoid global REVERT_TIMEOUT static race conditions.
