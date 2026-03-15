mod common;

use common::TestServer;

#[tokio::test]
async fn test_system_status() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/system/status")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let status: serde_json::Value = resp.json().await.unwrap();
    assert!(status["version"].is_string());
    assert_eq!(status["ebpf_loaded"], false);
    assert!(status["uptime_seconds"].as_u64().is_some());
}

#[tokio::test]
async fn test_system_interfaces() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/system/interfaces")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let interfaces: Vec<serde_json::Value> = resp.json().await.unwrap();
    // Should have at least loopback
    assert!(!interfaces.is_empty());
}

#[tokio::test]
async fn test_apply_config() {
    let server = TestServer::start().await;
    let resp = server.client.post(server.url("/api/v1/system/apply")).send().await.unwrap();
    assert!(resp.status().is_success());
    let result: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(result["status"], "applied");
    assert!(result["rules"].is_number());
}

#[tokio::test]
async fn test_prometheus_metrics() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/metrics")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    // Metrics endpoint should return text (may be empty in demo mode)
    let _body = resp.text().await.unwrap();
}

#[tokio::test]
async fn test_conntrack_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/conntrack")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let result: serde_json::Value = resp.json().await.unwrap();
    // Conntrack returns paginated object with entries array
    assert!(result["entries"].as_array().unwrap().is_empty());
    assert_eq!(result["total"], 0);
}

#[tokio::test]
async fn test_logs_empty() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/logs")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let result: serde_json::Value = resp.json().await.unwrap();
    // Logs returns paginated object with entries array
    assert!(result["entries"].as_array().unwrap().is_empty());
    assert_eq!(result["total"], 0);
}

#[tokio::test]
async fn test_logs_with_filters() {
    let server = TestServer::start().await;
    let resp = server.client.get(server.url("/api/v1/logs?src_ip=10.0.0.1&protocol=TCP&limit=10&offset=0"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
}
