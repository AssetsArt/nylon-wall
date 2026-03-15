mod common;

use common::{TestServer, sample_rule};
use futures_util::StreamExt;

#[tokio::test]
async fn test_ws_connect() {
    let server = TestServer::start().await;
    let ws_url = server.base_url.replace("http://", "ws://") + "/api/v1/ws/events";

    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
        .expect("Failed to connect WebSocket");

    // Connection should succeed
    drop(ws_stream);
}

#[tokio::test]
async fn test_ws_receives_rule_created() {
    let server = TestServer::start().await;
    let ws_url = server.base_url.replace("http://", "ws://") + "/api/v1/ws/events";

    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
        .expect("Failed to connect WebSocket");
    let (_, mut read) = ws_stream.split();

    // Give WebSocket time to fully establish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create a rule via HTTP
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);

    // Should receive a rule_created event within timeout
    let msg = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        read.next()
    ).await;

    match msg {
        Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text)))) => {
            let event: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(event["type"], "rule_created");
        }
        other => panic!("Expected text message with rule_created event, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_ws_receives_rule_deleted() {
    let server = TestServer::start().await;
    let ws_url = server.base_url.replace("http://", "ws://") + "/api/v1/ws/events";

    // Create and confirm a rule first
    let resp = server.client.post(server.url("/api/v1/rules"))
        .json(&sample_rule())
        .send().await.unwrap();
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_u64().unwrap();
    server.confirm().await;

    // Connect WebSocket
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
        .expect("Failed to connect WebSocket");
    let (_, mut read) = ws_stream.split();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Delete the rule
    server.client.delete(server.url(&format!("/api/v1/rules/{}", id)))
        .send().await.unwrap();

    // Should receive a rule_deleted event
    let msg = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        read.next()
    ).await;

    match msg {
        Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text)))) => {
            let event: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(event["type"], "rule_deleted");
        }
        other => panic!("Expected text message with rule_deleted event, got: {:?}", other),
    }
}
