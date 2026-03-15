mod common;

use common::{TestServer, sample_rule, sample_nat, sample_zone, create_and_confirm};

#[tokio::test]
async fn test_backup_empty() {
    let server = TestServer::start().await;
    let resp = server.client.post(server.url("/api/v1/system/backup")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let backup: serde_json::Value = resp.json().await.unwrap();
    assert!(backup["version"].is_string());
    assert!(backup["timestamp"].is_number());
    assert!(backup["rules"].as_array().unwrap().is_empty());
    assert!(backup["nat_entries"].as_array().unwrap().is_empty());
    assert!(backup["zones"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_backup_with_data() {
    let server = TestServer::start().await;

    // Create some data
    create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    create_and_confirm(&server, "/api/v1/nat", &sample_nat()).await;
    create_and_confirm(&server, "/api/v1/zones", &sample_zone()).await;

    let resp = server.client.post(server.url("/api/v1/system/backup")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let backup: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(backup["rules"].as_array().unwrap().len(), 1);
    assert_eq!(backup["nat_entries"].as_array().unwrap().len(), 1);
    assert_eq!(backup["zones"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_restore_roundtrip() {
    let server = TestServer::start().await;

    // Create initial data
    create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;

    // Backup
    let resp = server.client.post(server.url("/api/v1/system/backup")).send().await.unwrap();
    let backup: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(backup["rules"].as_array().unwrap().len(), 1);

    // Create more data
    create_and_confirm(&server, "/api/v1/nat", &sample_nat()).await;

    // Verify we have both
    let resp = server.client.get(server.url("/api/v1/nat")).send().await.unwrap();
    let nats: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(nats.len(), 1);

    // Restore from backup (which didn't have NAT)
    let resp = server.client.post(server.url("/api/v1/system/restore"))
        .json(&backup)
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    // Verify NAT is gone (restored state had no NAT)
    let resp = server.client.get(server.url("/api/v1/nat")).send().await.unwrap();
    let nats: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(nats.is_empty());

    // Verify rules still exist (were in backup)
    let resp = server.client.get(server.url("/api/v1/rules")).send().await.unwrap();
    let rules: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(rules.len(), 1);
}

#[tokio::test]
async fn test_backup_restore_full_roundtrip() {
    let server = TestServer::start().await;

    // Create data in all categories
    create_and_confirm(&server, "/api/v1/rules", &sample_rule()).await;
    create_and_confirm(&server, "/api/v1/nat", &sample_nat()).await;
    create_and_confirm(&server, "/api/v1/zones", &sample_zone()).await;

    // Backup
    let resp = server.client.post(server.url("/api/v1/system/backup")).send().await.unwrap();
    let backup1: serde_json::Value = resp.json().await.unwrap();

    // Restore same backup
    let resp = server.client.post(server.url("/api/v1/system/restore"))
        .json(&backup1)
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    // Backup again
    let resp = server.client.post(server.url("/api/v1/system/backup")).send().await.unwrap();
    let backup2: serde_json::Value = resp.json().await.unwrap();

    // Same data counts
    assert_eq!(
        backup1["rules"].as_array().unwrap().len(),
        backup2["rules"].as_array().unwrap().len()
    );
    assert_eq!(
        backup1["nat_entries"].as_array().unwrap().len(),
        backup2["nat_entries"].as_array().unwrap().len()
    );
}
