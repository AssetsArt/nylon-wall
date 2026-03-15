use std::sync::Arc;
use reqwest::Client;
use tempfile::TempDir;

pub struct TestServer {
    pub base_url: String,
    pub client: Client,
    _temp_dir: TempDir,
    _server_handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub async fn start() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("slatedb");
        std::fs::create_dir_all(&db_path).unwrap();

        let state = nylon_wall_daemon::create_test_state(db_path.to_str().unwrap()).await;

        // Set a long revert timeout so tests don't auto-revert unexpectedly
        nylon_wall_daemon::changeset::set_revert_timeout(300);

        let app = nylon_wall_daemon::api::build_router(Arc::clone(&state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);

        // Spawn auto-revert task (needed for change management tests)
        nylon_wall_daemon::changeset::spawn_auto_revert_task(Arc::clone(&state));

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            base_url,
            client: Client::new(),
            _temp_dir: temp_dir,
            _server_handle: handle,
        }
    }

    /// Start with a short revert timeout (for auto-revert tests).
    pub async fn start_with_revert_timeout(secs: u64) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("slatedb");
        std::fs::create_dir_all(&db_path).unwrap();

        let state = nylon_wall_daemon::create_test_state(db_path.to_str().unwrap()).await;
        nylon_wall_daemon::changeset::set_revert_timeout(secs);

        let app = nylon_wall_daemon::api::build_router(Arc::clone(&state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);

        nylon_wall_daemon::changeset::spawn_auto_revert_task(Arc::clone(&state));

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            base_url,
            client: Client::new(),
            _temp_dir: temp_dir,
            _server_handle: handle,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// POST to /api/v1/changes/confirm
    pub async fn confirm(&self) {
        let resp = self.client.post(self.url("/api/v1/changes/confirm"))
            .send().await.unwrap();
        assert!(resp.status().is_success(), "confirm failed: {}", resp.status());
    }
}

// === Sample Data Factories ===

pub fn sample_rule() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "name": "Test Rule",
        "priority": 100,
        "direction": "Ingress",
        "enabled": true,
        "src_ip": null,
        "dst_ip": null,
        "src_port": null,
        "dst_port": null,
        "protocol": null,
        "interface": null,
        "action": "Allow",
        "rate_limit_pps": null,
        "hit_count": 0,
        "created_at": 0,
        "updated_at": 0
    })
}

pub fn sample_rule_with_name(name: &str) -> serde_json::Value {
    let mut r = sample_rule();
    r["name"] = serde_json::json!(name);
    r
}

pub fn sample_nat() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "nat_type": "Masquerade",
        "enabled": true,
        "src_network": "192.168.1.0/24",
        "dst_network": null,
        "protocol": null,
        "dst_port": null,
        "in_interface": "eth1",
        "out_interface": "eth0",
        "translate_ip": null,
        "translate_port": null
    })
}

pub fn sample_route() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "destination": "10.0.0.0/8",
        "gateway": "192.168.1.1",
        "interface": "eth0",
        "metric": 100,
        "table": 254,
        "enabled": true
    })
}

pub fn sample_policy_route() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "src_ip": "192.168.1.0/24",
        "dst_ip": null,
        "src_port": null,
        "protocol": null,
        "route_table": 100,
        "priority": 100
    })
}

pub fn sample_zone() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "name": "LAN",
        "interfaces": ["eth1"],
        "default_policy": "Allow"
    })
}

pub fn sample_policy() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "name": "Allow LAN to WAN",
        "enabled": true,
        "from_zone": "LAN",
        "to_zone": "WAN",
        "src_ip": null,
        "dst_ip": null,
        "dst_port": null,
        "protocol": null,
        "schedule": null,
        "action": "Allow",
        "log": false,
        "priority": 100
    })
}

pub fn sample_sni_rule() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "domain": "facebook.com",
        "action": "Block",
        "enabled": true,
        "hit_count": 0,
        "category": "social"
    })
}

pub fn sample_dhcp_pool() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "interface": "eth1",
        "enabled": true,
        "subnet": "192.168.1.0/24",
        "range_start": "192.168.1.100",
        "range_end": "192.168.1.200",
        "gateway": "192.168.1.1",
        "dns_servers": ["8.8.8.8", "8.8.4.4"],
        "domain_name": "local",
        "lease_time": 3600
    })
}

pub fn sample_dhcp_reservation() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "pool_id": 1,
        "mac": "AA:BB:CC:DD:EE:FF",
        "ip": "192.168.1.50",
        "hostname": "my-server"
    })
}

pub fn sample_dhcp_client() -> serde_json::Value {
    serde_json::json!({
        "id": 0,
        "interface": "eth0",
        "enabled": false,
        "hostname": "nylon-wall"
    })
}

/// Create a resource via POST, confirm the change, and return the response body.
pub async fn create_and_confirm(server: &TestServer, path: &str, body: &serde_json::Value) -> serde_json::Value {
    let resp = server.client.post(server.url(path))
        .json(body)
        .send().await.unwrap();
    assert!(resp.status().is_success(), "create at {} failed: {}", path, resp.status());
    let val: serde_json::Value = resp.json().await.unwrap();
    server.confirm().await;
    val
}
