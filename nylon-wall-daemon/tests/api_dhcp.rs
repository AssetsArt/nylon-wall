mod common;

use common::{TestServer, sample_dhcp_pool, sample_dhcp_reservation, sample_dhcp_client, create_and_confirm};

#[tokio::test]
async fn test_dhcp_pool_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/dhcp/pools")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let pools: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(pools.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/dhcp/pools", &sample_dhcp_pool()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["subnet"], "192.168.1.0/24");
    assert_eq!(created["interface"], "eth1");

    // Get by ID
    let resp = server.client.get(server.url(&format!("/api/v1/dhcp/pools/{}", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    // Update
    let mut updated = created.clone();
    updated["lease_time"] = serde_json::json!(7200);
    let resp = server.client.put(server.url(&format!("/api/v1/dhcp/pools/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete (may return 200 or 204)
    let resp = server.client.delete(server.url(&format!("/api/v1/dhcp/pools/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;
}

#[tokio::test]
async fn test_dhcp_pool_toggle() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/dhcp/pools", &sample_dhcp_pool()).await;
    let id = created["id"].as_u64().unwrap();

    let resp = server.client.post(server.url(&format!("/api/v1/dhcp/pools/{}/toggle", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
}

#[tokio::test]
async fn test_dhcp_reservation_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/dhcp/reservations")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let reservations: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(reservations.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/dhcp/reservations", &sample_dhcp_reservation()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["mac"], "AA:BB:CC:DD:EE:FF");
    assert_eq!(created["ip"], "192.168.1.50");

    // Update
    let mut updated = created.clone();
    updated["hostname"] = serde_json::json!("new-hostname");
    let resp = server.client.put(server.url(&format!("/api/v1/dhcp/reservations/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/dhcp/reservations/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;
}

#[tokio::test]
async fn test_dhcp_client_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/dhcp/clients")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let clients: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(clients.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/dhcp/clients", &sample_dhcp_client()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["interface"], "eth0");

    // Update
    let mut updated = created.clone();
    updated["hostname"] = serde_json::json!("router");
    let resp = server.client.put(server.url(&format!("/api/v1/dhcp/clients/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/dhcp/clients/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;
}

#[tokio::test]
async fn test_dhcp_leases_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/dhcp/leases")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let leases: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(leases.is_empty());
}

#[tokio::test]
async fn test_dhcp_client_status() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/dhcp/clients/status")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
}
