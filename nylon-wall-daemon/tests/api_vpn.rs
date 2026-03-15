mod common;

use common::{TestServer, sample_wg_server, sample_wg_peer};

#[tokio::test]
async fn test_wg_server_default() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/vpn/server")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.get("listen_port").is_some());
}

#[tokio::test]
async fn test_wg_server_update() {
    let server = TestServer::start().await;

    let resp = server.client.put(server.url("/api/v1/vpn/server"))
        .json(&sample_wg_server())
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["listen_port"], 51820);
    assert_eq!(body["address"], "10.0.100.1/24");
    assert_eq!(body["endpoint"], "vpn.example.com");
    // Keys should be auto-generated
    assert!(!body["private_key"].as_str().unwrap().is_empty());
    assert!(!body["public_key"].as_str().unwrap().is_empty());

    // Verify persisted
    let resp = server.client.get(server.url("/api/v1/vpn/server")).send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["listen_port"], 51820);
}

#[tokio::test]
async fn test_wg_server_toggle() {
    let server = TestServer::start().await;

    // Set up server first
    server.client.put(server.url("/api/v1/vpn/server"))
        .json(&sample_wg_server())
        .send().await.unwrap();

    // Toggle on
    let resp = server.client.post(server.url("/api/v1/vpn/server/toggle"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["enabled"], true);

    // Toggle off
    let resp = server.client.post(server.url("/api/v1/vpn/server/toggle"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["enabled"], false);
}

#[tokio::test]
async fn test_wg_peers_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/vpn/peers")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let peers: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(peers.is_empty());
}

#[tokio::test]
async fn test_wg_peer_crud_lifecycle() {
    let server = TestServer::start().await;

    // Create (WG peer doesn't use changeset)
    let resp = server.client.post(server.url("/api/v1/vpn/peers"))
        .json(&sample_wg_peer())
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();
    assert!(id > 0);
    assert_eq!(created["name"], "Phone");
    assert_eq!(created["allowed_ips"], "10.0.100.2/32");
    assert_eq!(created["enabled"], true);
    // Keys should be auto-generated
    assert!(!created["public_key"].as_str().unwrap().is_empty());

    // List (keys are masked)
    let resp = server.client.get(server.url("/api/v1/vpn/peers")).send().await.unwrap();
    let peers: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(peers.len(), 1);

    // Update
    let mut updated = created.clone();
    updated["name"] = serde_json::json!("Laptop");
    updated["persistent_keepalive"] = serde_json::json!(30);
    let resp = server.client.put(server.url(&format!("/api/v1/vpn/peers/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Laptop");
    assert_eq!(body["persistent_keepalive"], 30);

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/vpn/peers/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());

    let resp = server.client.get(server.url("/api/v1/vpn/peers")).send().await.unwrap();
    let peers: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(peers.is_empty());
}

#[tokio::test]
async fn test_wg_peer_toggle() {
    let server = TestServer::start().await;

    let resp = server.client.post(server.url("/api/v1/vpn/peers"))
        .json(&sample_wg_peer())
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["enabled"], true);

    let resp = server.client.post(server.url(&format!("/api/v1/vpn/peers/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let toggled: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(toggled["enabled"], false);
}

#[tokio::test]
async fn test_wg_peer_config_download() {
    let server = TestServer::start().await;

    // Set up server config first
    server.client.put(server.url("/api/v1/vpn/server"))
        .json(&sample_wg_server())
        .send().await.unwrap();

    // Create peer
    let resp = server.client.post(server.url("/api/v1/vpn/peers"))
        .json(&sample_wg_peer())
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();

    // Download config
    let resp = server.client.get(server.url(&format!("/api/v1/vpn/peers/{}/config", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/plain"));
    let body = resp.text().await.unwrap();
    assert!(body.contains("[Interface]"));
    assert!(body.contains("[Peer]"));
}
