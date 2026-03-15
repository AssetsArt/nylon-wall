mod common;

use common::{TestServer, sample_l4proxy, create_and_confirm};

#[tokio::test]
async fn test_list_l4proxy_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/l4proxy/rules")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(rules.is_empty());
}

#[tokio::test]
async fn test_l4proxy_crud_lifecycle() {
    let server = TestServer::start().await;

    // Create
    let created = create_and_confirm(&server, "/api/v1/l4proxy/rules", &sample_l4proxy()).await;
    let id = created["id"].as_u64().unwrap();
    assert!(id > 0);
    assert_eq!(created["name"], "Web Backend");
    assert_eq!(created["protocol"], "TCP");
    assert_eq!(created["listen_port"], 8080);
    assert_eq!(created["upstream_targets"].as_array().unwrap().len(), 2);
    assert_eq!(created["load_balance"], "RoundRobin");
    assert_eq!(created["enabled"], true);

    // List
    let resp = server.client.get(server.url("/api/v1/l4proxy/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(rules.len(), 1);

    // Update
    let mut updated = created.clone();
    updated["name"] = serde_json::json!("Web Backend v2");
    updated["load_balance"] = serde_json::json!("IpHash");
    let resp = server.client.put(server.url(&format!("/api/v1/l4proxy/rules/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Web Backend v2");
    assert_eq!(body["load_balance"], "IpHash");
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/l4proxy/rules/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/l4proxy/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(rules.is_empty());
}

#[tokio::test]
async fn test_l4proxy_toggle() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/l4proxy/rules", &sample_l4proxy()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["enabled"], true);

    // Toggle off
    let resp = server.client.post(server.url(&format!("/api/v1/l4proxy/rules/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let toggled: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(toggled["enabled"], false);
    server.confirm().await;

    // Toggle back on
    let resp = server.client.post(server.url(&format!("/api/v1/l4proxy/rules/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let toggled: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(toggled["enabled"], true);
    server.confirm().await;
}

#[tokio::test]
async fn test_l4proxy_duplicate_listen_port() {
    let server = TestServer::start().await;
    create_and_confirm(&server, "/api/v1/l4proxy/rules", &sample_l4proxy()).await;

    // Try creating another rule with the same listen port + protocol + address
    let resp = server.client.post(server.url("/api/v1/l4proxy/rules"))
        .json(&sample_l4proxy())
        .send().await.unwrap();
    assert_eq!(resp.status(), 409); // Conflict
}

#[tokio::test]
async fn test_l4proxy_empty_upstreams_rejected() {
    let server = TestServer::start().await;

    let mut rule = sample_l4proxy();
    rule["upstream_targets"] = serde_json::json!([]);

    let resp = server.client.post(server.url("/api/v1/l4proxy/rules"))
        .json(&rule)
        .send().await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_l4proxy_udp_type() {
    let server = TestServer::start().await;

    let rule = serde_json::json!({
        "id": 0,
        "name": "DNS LB",
        "protocol": "UDP",
        "listen_address": "0.0.0.0",
        "listen_port": 5353,
        "upstream_targets": [
            { "address": "8.8.8.8", "port": 53, "weight": 1 },
            { "address": "8.8.4.4", "port": 53, "weight": 2 }
        ],
        "load_balance": "IpHash",
        "enabled": true
    });

    let created = create_and_confirm(&server, "/api/v1/l4proxy/rules", &rule).await;
    assert_eq!(created["protocol"], "UDP");
    assert_eq!(created["listen_port"], 5353);
    assert_eq!(created["load_balance"], "IpHash");
    assert_eq!(created["upstream_targets"][1]["weight"], 2);
}
