mod common;

use common::{TestServer, sample_rule};

/// This test is in its own file because `set_revert_timeout` uses a global static
/// and we need to avoid races with other tests that set it to 300s.
#[tokio::test]
async fn test_auto_revert_on_timeout() {
    let server = TestServer::start_with_revert_timeout(2).await;

    // Create rule
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();

    // Wait for auto-revert (timeout + buffer)
    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    // Rule should be auto-reverted (gone)
    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);

    // No longer pending
    let resp = server.client.get(server.url("/api/v1/changes/pending")).send().await.unwrap();
    let status: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status["pending"], false);
}
