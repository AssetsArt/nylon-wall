mod common;

use common::{TestServer, sample_rule, sample_rule_with_name, create_and_confirm};

#[tokio::test]
async fn test_list_rules_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/rules")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(rules.is_empty());
}

#[tokio::test]
async fn test_create_and_get_rule() {
    let server = TestServer::start().await;

    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();
    assert!(id > 0);

    server.confirm().await;

    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let fetched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(fetched["name"], "Test Rule");
    assert_eq!(fetched["action"], "Allow");
}

#[tokio::test]
async fn test_create_rule_auto_id() {
    let server = TestServer::start().await;

    let r1 = create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Rule A")).await;
    let r2 = create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Rule B")).await;

    assert!(r2["id"].as_u64().unwrap() > r1["id"].as_u64().unwrap());
}

#[tokio::test]
async fn test_get_rule_not_found() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/rules/999"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_update_rule() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    let id = created["id"].as_u64().unwrap();

    let mut updated = created.clone();
    updated["name"] = serde_json::json!("Updated Rule");
    updated["action"] = serde_json::json!("Drop");

    let resp = server.client.put(server.url(&format!("/api/v1/rules/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    let fetched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(fetched["name"], "Updated Rule");
    assert_eq!(fetched["action"], "Drop");
}

#[tokio::test]
async fn test_delete_rule() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    let id = created["id"].as_u64().unwrap();

    let resp = server.client.delete(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_toggle_rule() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["enabled"], true);

    let resp = server.client.post(server.url(&format!("/api/v1/rules/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    let resp = server.client.get(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();
    let fetched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(fetched["enabled"], false);
}

#[tokio::test]
async fn test_reorder_rules() {
    let server = TestServer::start().await;

    let r1 = create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Rule 1")).await;
    let r2 = create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Rule 2")).await;
    let r3 = create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Rule 3")).await;

    let id1 = r1["id"].as_u64().unwrap().to_string();
    let id2 = r2["id"].as_u64().unwrap().to_string();
    let id3 = r3["id"].as_u64().unwrap().to_string();

    // Reorder: 3, 1, 2
    let resp = server.client.post(server.url("/api/v1/rules/reorder"))
        .json(&serde_json::json!({ "rule_ids": [id3, id1, id2] }))
        .send().await.unwrap();
    assert!(resp.status().is_success());

    let resp = server.client.get(server.url("/api/v1/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(rules.len(), 3);
}

#[tokio::test]
async fn test_create_rule_all_fields() {
    let server = TestServer::start().await;

    let rule = serde_json::json!({
        "id": 0,
        "name": "Full Rule",
        "priority": 50,
        "direction": "Egress",
        "enabled": true,
        "src_ip": "10.0.0.0/8",
        "dst_ip": "192.168.1.0/24",
        "src_port": { "start": 1024, "end": 65535 },
        "dst_port": { "start": 80, "end": 443 },
        "protocol": "TCP",
        "interface": "eth0",
        "action": "RateLimit",
        "rate_limit_pps": 1000,
        "hit_count": 0,
        "created_at": 0,
        "updated_at": 0
    });

    let created = create_and_confirm(&server, "/api/v1/rules", &rule).await;
    assert_eq!(created["direction"], "Egress");
    assert_eq!(created["src_ip"], "10.0.0.0/8");
    assert_eq!(created["protocol"], "TCP");
    assert_eq!(created["action"], "RateLimit");
    assert_eq!(created["rate_limit_pps"], 1000);
}

#[tokio::test]
async fn test_list_rules_returns_created() {
    let server = TestServer::start().await;

    create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Alpha")).await;
    create_and_confirm(&server, "/api/v1/rules", &sample_rule_with_name("Beta")).await;

    let resp = server.client.get(server.url("/api/v1/rules")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(rules.len(), 2);
}
