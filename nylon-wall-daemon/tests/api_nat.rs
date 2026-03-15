mod common;

use common::{TestServer, sample_nat, create_and_confirm};

#[tokio::test]
async fn test_list_nat_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/nat")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn test_nat_crud_lifecycle() {
    let server = TestServer::start().await;

    // Create
    let created = create_and_confirm(&server, "/api/v1/nat", &sample_nat()).await;
    let id = created["id"].as_u64().unwrap();
    assert!(id > 0);
    assert_eq!(created["nat_type"], "Masquerade");

    // List
    let resp = server.client.get(server.url("/api/v1/nat")).send().await.unwrap();
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(entries.len(), 1);

    // Update
    let mut updated = created.clone();
    updated["src_network"] = serde_json::json!("10.0.0.0/8");
    let resp = server.client.put(server.url(&format!("/api/v1/nat/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/nat/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/nat")).send().await.unwrap();
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn test_nat_toggle() {
    let server = TestServer::start().await;
    let created = create_and_confirm(&server, "/api/v1/nat", &sample_nat()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["enabled"], true);

    let resp = server.client.post(server.url(&format!("/api/v1/nat/{}/toggle", id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/nat")).send().await.unwrap();
    let entries: Vec<serde_json::Value> = resp.json().await.unwrap();
    let entry = entries.iter().find(|e| e["id"].as_u64().unwrap() == id).unwrap();
    assert_eq!(entry["enabled"], false);
}

#[tokio::test]
async fn test_nat_snat_type() {
    let server = TestServer::start().await;

    let nat = serde_json::json!({
        "id": 0,
        "nat_type": "SNAT",
        "enabled": true,
        "src_network": "192.168.1.0/24",
        "dst_network": null,
        "protocol": null,
        "dst_port": null,
        "in_interface": null,
        "out_interface": "eth0",
        "translate_ip": "203.0.113.1",
        "translate_port": null
    });

    let created = create_and_confirm(&server, "/api/v1/nat", &nat).await;
    assert_eq!(created["nat_type"], "SNAT");
    assert_eq!(created["translate_ip"], "203.0.113.1");
}
