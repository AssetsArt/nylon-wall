mod common;

use common::{TestServer, sample_zone, sample_policy, create_and_confirm};

#[tokio::test]
async fn test_zone_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/zones")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let zones: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(zones.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/zones", &sample_zone()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["name"], "LAN");

    // Update
    let mut updated = created.clone();
    updated["name"] = serde_json::json!("DMZ");
    updated["interfaces"] = serde_json::json!(["eth2"]);
    let resp = server.client.put(server.url(&format!("/api/v1/zones/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Verify update
    let resp = server.client.get(server.url("/api/v1/zones")).send().await.unwrap();
    let zones: Vec<serde_json::Value> = resp.json().await.unwrap();
    let zone = zones.iter().find(|z| z["id"].as_u64().unwrap() == id).unwrap();
    assert_eq!(zone["name"], "DMZ");

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/zones/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/zones")).send().await.unwrap();
    let zones: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(zones.is_empty());
}

#[tokio::test]
async fn test_policy_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/policies")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let policies: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(policies.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/policies", &sample_policy()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["name"], "Allow LAN to WAN");
    assert_eq!(created["action"], "Allow");

    // Update
    let mut updated = created.clone();
    updated["action"] = serde_json::json!("Drop");
    updated["log"] = serde_json::json!(true);
    let resp = server.client.put(server.url(&format!("/api/v1/policies/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/policies/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;
}

#[tokio::test]
async fn test_policy_with_schedule() {
    let server = TestServer::start().await;

    let policy = serde_json::json!({
        "id": 0,
        "name": "Working Hours Only",
        "enabled": true,
        "from_zone": "LAN",
        "to_zone": "WAN",
        "src_ip": null,
        "dst_ip": null,
        "dst_port": null,
        "protocol": null,
        "schedule": {
            "days": [0, 1, 2, 3, 4],
            "start_time": "09:00",
            "end_time": "17:00"
        },
        "action": "Allow",
        "log": false,
        "priority": 50
    });

    let created = create_and_confirm(&server, "/api/v1/policies", &policy).await;
    assert!(created["schedule"].is_object());
    assert_eq!(created["schedule"]["start_time"], "09:00");
    assert_eq!(created["schedule"]["end_time"], "17:00");
}
