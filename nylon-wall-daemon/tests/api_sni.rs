mod common;

use common::{TestServer, sample_sni_rule, create_and_confirm};

#[tokio::test]
async fn test_sni_rule_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/tls/sni/rules")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(rules.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/tls/sni/rules", &sample_sni_rule()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["domain"], "facebook.com");
    assert_eq!(created["action"], "Block");

    // List again
    let resp = server.client.get(server.url("/api/v1/tls/sni/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(rules.len(), 1);

    // Update
    let mut updated = created.clone();
    updated["action"] = serde_json::json!("Log");
    let resp = server.client.put(server.url(&format!("/api/v1/tls/sni/rules/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/tls/sni/rules/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/tls/sni/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(rules.is_empty());
}

#[tokio::test]
async fn test_sni_toggle_rule() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/tls/sni/rules", &sample_sni_rule()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["enabled"], true);

    let resp = server.client.post(server.url(&format!("/api/v1/tls/sni/rules/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/tls/sni/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    let rule = rules.iter().find(|r| r["id"].as_u64().unwrap() == id).unwrap();
    assert_eq!(rule["enabled"], false);
}

#[tokio::test]
async fn test_sni_global_toggle() {
    let server = TestServer::start().await;

    // Toggle SNI filtering on
    let resp = server.client.post(server.url("/api/v1/tls/sni/toggle"))
        .send().await.unwrap();
    assert!(resp.status().is_success());
}

#[tokio::test]
async fn test_sni_stats() {
    let server = TestServer::start().await;

    let resp = server.client.get(server.url("/api/v1/tls/sni/stats")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let stats: serde_json::Value = resp.json().await.unwrap();
    // In demo mode, all stats should be zero
    assert_eq!(stats["total_inspected"], 0);
    assert_eq!(stats["total_blocked"], 0);
}

#[tokio::test]
async fn test_sni_wildcard_rule() {
    let server = TestServer::start().await;

    let rule = serde_json::json!({
        "id": 0,
        "domain": "*.tiktok.com",
        "action": "Block",
        "enabled": true,
        "hit_count": 0,
        "category": "social"
    });

    let created = create_and_confirm(&server, "/api/v1/tls/sni/rules", &rule).await;
    assert_eq!(created["domain"], "*.tiktok.com");
}
