mod common;

use common::{TestServer, sample_vlan, sample_bridge, create_and_confirm};

// === VLAN Tests ===

#[tokio::test]
async fn test_list_vlans_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/vnet/vlans")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let vlans: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(vlans.is_empty());
}

#[tokio::test]
async fn test_vlan_crud_lifecycle() {
    let server = TestServer::start().await;

    // Create
    let created = create_and_confirm(&server, "/api/v1/vnet/vlans", &sample_vlan()).await;
    let id = created["id"].as_u64().unwrap();
    assert!(id > 0);
    assert_eq!(created["vlan_id"], 100);
    assert_eq!(created["parent_interface"], "eth0");
    assert_eq!(created["enabled"], true);

    // List
    let resp = server.client.get(server.url("/api/v1/vnet/vlans")).send().await.unwrap();
    let vlans: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(vlans.len(), 1);

    // Update
    let mut updated = created.clone();
    updated["ip_address"] = serde_json::json!("10.10.100.2/24");
    let resp = server.client.put(server.url(&format!("/api/v1/vnet/vlans/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ip_address"], "10.10.100.2/24");
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/vnet/vlans/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/vnet/vlans")).send().await.unwrap();
    let vlans: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(vlans.is_empty());
}

#[tokio::test]
async fn test_vlan_toggle() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/vnet/vlans", &sample_vlan()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["enabled"], true);

    let resp = server.client.post(server.url(&format!("/api/v1/vnet/vlans/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let toggled: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(toggled["enabled"], false);
    server.confirm().await;
}

#[tokio::test]
async fn test_vlan_duplicate_rejected() {
    let server = TestServer::start().await;
    create_and_confirm(&server, "/api/v1/vnet/vlans", &sample_vlan()).await;

    // Same vlan_id + parent_interface should be rejected
    let resp = server.client.post(server.url("/api/v1/vnet/vlans"))
        .json(&sample_vlan())
        .send().await.unwrap();
    assert_eq!(resp.status(), 409);
}

// === Bridge Tests ===

#[tokio::test]
async fn test_list_bridges_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/vnet/bridges")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let bridges: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(bridges.is_empty());
}

#[tokio::test]
async fn test_bridge_crud_lifecycle() {
    let server = TestServer::start().await;

    // Create
    let created = create_and_confirm(&server, "/api/v1/vnet/bridges", &sample_bridge()).await;
    let id = created["id"].as_u64().unwrap();
    assert!(id > 0);
    assert_eq!(created["name"], "br0");
    assert_eq!(created["ports"].as_array().unwrap().len(), 2);
    assert_eq!(created["stp_enabled"], false);
    assert_eq!(created["enabled"], true);

    // List
    let resp = server.client.get(server.url("/api/v1/vnet/bridges")).send().await.unwrap();
    let bridges: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(bridges.len(), 1);

    // Update
    let mut updated = created.clone();
    updated["stp_enabled"] = serde_json::json!(true);
    updated["ports"] = serde_json::json!(["eth1", "eth2", "eth3"]);
    let resp = server.client.put(server.url(&format!("/api/v1/vnet/bridges/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["stp_enabled"], true);
    assert_eq!(body["ports"].as_array().unwrap().len(), 3);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/vnet/bridges/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/vnet/bridges")).send().await.unwrap();
    let bridges: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(bridges.is_empty());
}

#[tokio::test]
async fn test_bridge_toggle() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/vnet/bridges", &sample_bridge()).await;
    let id = created["id"].as_u64().unwrap();

    let resp = server.client.post(server.url(&format!("/api/v1/vnet/bridges/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let toggled: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(toggled["enabled"], false);
    server.confirm().await;
}

#[tokio::test]
async fn test_bridge_duplicate_name_rejected() {
    let server = TestServer::start().await;
    create_and_confirm(&server, "/api/v1/vnet/bridges", &sample_bridge()).await;

    // Same bridge name should be rejected
    let resp = server.client.post(server.url("/api/v1/vnet/bridges"))
        .json(&sample_bridge())
        .send().await.unwrap();
    assert_eq!(resp.status(), 409);
}
