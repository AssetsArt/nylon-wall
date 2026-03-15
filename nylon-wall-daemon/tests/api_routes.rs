mod common;

use common::{TestServer, sample_route, sample_policy_route, create_and_confirm};

#[tokio::test]
async fn test_route_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/routes")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let routes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(routes.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/routes", &sample_route()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["destination"], "10.0.0.0/8");

    // Update
    let mut updated = created.clone();
    updated["metric"] = serde_json::json!(200);
    let resp = server.client.put(server.url(&format!("/api/v1/routes/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/routes/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/routes")).send().await.unwrap();
    let routes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(routes.is_empty());
}

#[tokio::test]
async fn test_policy_route_crud() {
    let server = TestServer::start().await;

    // List empty
    let resp = server.client.get(server.url("/api/v1/routes/policy")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let routes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(routes.is_empty());

    // Create
    let created = create_and_confirm(&server, "/api/v1/routes/policy", &sample_policy_route()).await;
    let id = created["id"].as_u64().unwrap();
    assert_eq!(created["route_table"], 100);

    // Update
    let mut updated = created.clone();
    updated["priority"] = serde_json::json!(200);
    let resp = server.client.put(server.url(&format!("/api/v1/routes/policy/{}", id)))
        .json(&updated)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    server.confirm().await;

    // Delete
    let resp = server.client.delete(server.url(&format!("/api/v1/routes/policy/{}", id)))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    server.confirm().await;

    let resp = server.client.get(server.url("/api/v1/routes/policy")).send().await.unwrap();
    let routes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(routes.is_empty());
}

#[tokio::test]
async fn test_routes_independent() {
    let server = TestServer::start().await;

    // Create one static route and one policy route
    create_and_confirm(&server, "/api/v1/routes", &sample_route()).await;
    create_and_confirm(&server, "/api/v1/routes/policy", &sample_policy_route()).await;

    // Verify they are separate
    let resp = server.client.get(server.url("/api/v1/routes")).send().await.unwrap();
    let static_routes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(static_routes.len(), 1);

    let resp = server.client.get(server.url("/api/v1/routes/policy")).send().await.unwrap();
    let policy_routes: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(policy_routes.len(), 1);
}
