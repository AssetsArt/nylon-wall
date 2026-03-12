use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
};
use nylon_wall_common::conntrack::{ConnState, ConntrackInfo};
use nylon_wall_common::log::PacketLog;
use nylon_wall_common::nat::NatEntry;
use nylon_wall_common::route::{PolicyRoute, Route};
use nylon_wall_common::rule::FirewallRule;
use nylon_wall_common::zone::{NetworkPolicy, Zone};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::AppState;

type AppResult<T> = Result<Json<T>, (StatusCode, String)>;

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Deserialize)]
struct ReorderRequest {
    rule_ids: Vec<String>,
}

#[derive(Deserialize)]
struct LogQuery {
    src_ip: Option<String>,
    dst_ip: Option<String>,
    protocol: Option<String>,
    action: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct NetworkInterface {
    name: String,
    mac: String,
    ip: String,
    status: String,
    mtu: u32,
}

#[derive(Serialize, Deserialize)]
struct BackupData {
    version: String,
    timestamp: i64,
    rules: Vec<serde_json::Value>,
    nat_entries: Vec<serde_json::Value>,
    routes: Vec<serde_json::Value>,
    zones: Vec<serde_json::Value>,
    policies: Vec<serde_json::Value>,
}

pub async fn serve(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        // Rules (reorder must be before {id} to avoid matching "reorder" as an id)
        .route("/api/v1/rules/reorder", post(reorder_rules))
        .route("/api/v1/rules", get(list_rules).post(create_rule))
        .route(
            "/api/v1/rules/{id}",
            get(get_rule).put(update_rule).delete(delete_rule),
        )
        .route("/api/v1/rules/{id}/toggle", post(toggle_rule))
        // NAT
        .route("/api/v1/nat", get(list_nat).post(create_nat))
        .route("/api/v1/nat/{id}", put(update_nat).delete(delete_nat))
        // Routes (policy routes must be before {id} to avoid "policy" being matched as an id)
        .route(
            "/api/v1/routes/policy",
            get(list_policy_routes).post(create_policy_route),
        )
        .route(
            "/api/v1/routes/policy/{id}",
            put(update_policy_route).delete(delete_policy_route),
        )
        .route("/api/v1/routes", get(list_routes).post(create_route))
        .route(
            "/api/v1/routes/{id}",
            put(update_route).delete(delete_route),
        )
        // Zones
        .route("/api/v1/zones", get(list_zones).post(create_zone))
        .route(
            "/api/v1/zones/{id}",
            put(update_zone).delete(delete_zone),
        )
        // Policies
        .route(
            "/api/v1/policies",
            get(list_policies).post(create_policy),
        )
        .route(
            "/api/v1/policies/{id}",
            put(update_policy).delete(delete_policy),
        )
        // Conntrack
        .route("/api/v1/conntrack", get(list_conntrack))
        // Logs
        .route("/api/v1/logs", get(list_logs))
        // System
        .route("/api/v1/system/status", get(system_status))
        .route("/api/v1/system/interfaces", get(list_interfaces))
        .route("/api/v1/system/backup", post(backup_data))
        .route("/api/v1/system/restore", post(restore_data))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

// === Rules ===

async fn list_rules(State(state): State<Arc<AppState>>) -> AppResult<Vec<FirewallRule>> {
    let results = state
        .db
        .scan_prefix::<FirewallRule>("rule:")
        .await
        .map_err(internal_error)?;
    let rules: Vec<FirewallRule> = results.into_iter().map(|(_, r)| r).collect();
    Ok(Json(rules))
}

async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(rule): Json<FirewallRule>,
) -> Result<(StatusCode, Json<FirewallRule>), (StatusCode, String)> {
    let key = format!("rule:{}", rule.id);
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("rule:", &key)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(rule)))
}

async fn get_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<FirewallRule> {
    let key = format!("rule:{}", id);
    match state
        .db
        .get::<FirewallRule>(&key)
        .await
        .map_err(internal_error)?
    {
        Some(rule) => Ok(Json(rule)),
        None => Err((StatusCode::NOT_FOUND, "Rule not found".to_string())),
    }
}

async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut rule): Json<FirewallRule>,
) -> AppResult<FirewallRule> {
    rule.id = id;
    let key = format!("rule:{}", id);
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    // Ensure it's in the index (idempotent)
    state
        .db
        .add_to_index("rule:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(rule))
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("rule:{}", id);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("rule:", &key)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<FirewallRule> {
    let key = format!("rule:{}", id);
    let mut rule: FirewallRule = state
        .db
        .get(&key)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Rule not found".to_string()))?;
    rule.enabled = !rule.enabled;
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    Ok(Json(rule))
}

// === NAT ===

async fn list_nat(State(state): State<Arc<AppState>>) -> AppResult<Vec<NatEntry>> {
    let results = state
        .db
        .scan_prefix::<NatEntry>("nat:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, n)| n).collect()))
}

async fn create_nat(
    State(state): State<Arc<AppState>>,
    Json(entry): Json<NatEntry>,
) -> Result<(StatusCode, Json<NatEntry>), (StatusCode, String)> {
    let key = format!("nat:{}", entry.id);
    state.db.put(&key, &entry).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("nat:", &key)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn update_nat(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut entry): Json<NatEntry>,
) -> AppResult<NatEntry> {
    entry.id = id;
    let key = format!("nat:{}", id);
    state.db.put(&key, &entry).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("nat:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(entry))
}

async fn delete_nat(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("nat:{}", id);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("nat:", &key)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// === Routes ===

async fn list_routes(State(state): State<Arc<AppState>>) -> AppResult<Vec<Route>> {
    let results = state
        .db
        .scan_prefix::<Route>("route:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, r)| r).collect()))
}

async fn create_route(
    State(state): State<Arc<AppState>>,
    Json(route): Json<Route>,
) -> Result<(StatusCode, Json<Route>), (StatusCode, String)> {
    let key = format!("route:{}", route.id);
    state.db.put(&key, &route).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("route:", &key)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(route)))
}

async fn update_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut route): Json<Route>,
) -> AppResult<Route> {
    route.id = id;
    let key = format!("route:{}", id);
    state.db.put(&key, &route).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("route:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(route))
}

async fn delete_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("route:{}", id);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("route:", &key)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// === Policy Routes ===

async fn list_policy_routes(State(state): State<Arc<AppState>>) -> AppResult<Vec<PolicyRoute>> {
    let results = state
        .db
        .scan_prefix::<PolicyRoute>("policy_route:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, r)| r).collect()))
}

async fn create_policy_route(
    State(state): State<Arc<AppState>>,
    Json(route): Json<PolicyRoute>,
) -> Result<(StatusCode, Json<PolicyRoute>), (StatusCode, String)> {
    let key = format!("policy_route:{}", route.id);
    state.db.put(&key, &route).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("policy_route:", &key)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(route)))
}

async fn update_policy_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut route): Json<PolicyRoute>,
) -> AppResult<PolicyRoute> {
    route.id = id;
    let key = format!("policy_route:{}", id);
    state.db.put(&key, &route).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("policy_route:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(route))
}

async fn delete_policy_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("policy_route:{}", id);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("policy_route:", &key)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// === Zones ===

async fn list_zones(State(state): State<Arc<AppState>>) -> AppResult<Vec<Zone>> {
    let results = state
        .db
        .scan_prefix::<Zone>("zone:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, z)| z).collect()))
}

async fn create_zone(
    State(state): State<Arc<AppState>>,
    Json(zone): Json<Zone>,
) -> Result<(StatusCode, Json<Zone>), (StatusCode, String)> {
    let key = format!("zone:{}", zone.id);
    state.db.put(&key, &zone).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("zone:", &key)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(zone)))
}

async fn update_zone(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut zone): Json<Zone>,
) -> AppResult<Zone> {
    zone.id = id;
    let key = format!("zone:{}", id);
    state.db.put(&key, &zone).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("zone:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(zone))
}

async fn delete_zone(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("zone:{}", id);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("zone:", &key)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// === Policies ===

async fn list_policies(State(state): State<Arc<AppState>>) -> AppResult<Vec<NetworkPolicy>> {
    let results = state
        .db
        .scan_prefix::<NetworkPolicy>("policy:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, p)| p).collect()))
}

async fn create_policy(
    State(state): State<Arc<AppState>>,
    Json(policy): Json<NetworkPolicy>,
) -> Result<(StatusCode, Json<NetworkPolicy>), (StatusCode, String)> {
    let key = format!("policy:{}", policy.id);
    state.db.put(&key, &policy).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("policy:", &key)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(policy)))
}

async fn update_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut policy): Json<NetworkPolicy>,
) -> AppResult<NetworkPolicy> {
    policy.id = id;
    let key = format!("policy:{}", id);
    state
        .db
        .put(&key, &policy)
        .await
        .map_err(internal_error)?;
    state
        .db
        .add_to_index("policy:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(policy))
}

async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("policy:{}", id);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("policy:", &key)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// === System ===

#[derive(serde::Serialize)]
struct SystemStatus {
    version: String,
    ebpf_loaded: bool,
    uptime_seconds: u64,
}

async fn system_status() -> Json<SystemStatus> {
    Json(SystemStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        ebpf_loaded: cfg!(target_os = "linux"),
        uptime_seconds: 0, // TODO: track actual uptime
    })
}

// === Rules Reorder ===

async fn reorder_rules(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReorderRequest>,
) -> Result<Json<Vec<FirewallRule>>, (StatusCode, String)> {
    let mut updated_rules = Vec::new();
    for (index, rule_id) in req.rule_ids.iter().enumerate() {
        let key = format!("rule:{}", rule_id);
        let mut rule: FirewallRule = state
            .db
            .get(&key)
            .await
            .map_err(internal_error)?
            .ok_or((
                StatusCode::NOT_FOUND,
                format!("Rule not found: {}", rule_id),
            ))?;
        rule.priority = (index * 10) as u32;
        state.db.put(&key, &rule).await.map_err(internal_error)?;
        updated_rules.push(rule);
    }
    Ok(Json(updated_rules))
}

// === Conntrack ===

async fn list_conntrack(
    State(_state): State<Arc<AppState>>,
) -> AppResult<Vec<ConntrackInfo>> {
    // No real eBPF data yet; return mock/sample entries
    let mock_entries = vec![
        ConntrackInfo {
            src_ip: "192.168.1.100".to_string(),
            dst_ip: "10.0.0.1".to_string(),
            src_port: 54321,
            dst_port: 443,
            protocol: "TCP".to_string(),
            state: ConnState::Established,
            packets_in: 1520,
            packets_out: 980,
            bytes_in: 2_048_000,
            bytes_out: 512_000,
            last_seen: chrono::Utc::now().timestamp() as u64,
            timeout: 300,
        },
        ConntrackInfo {
            src_ip: "192.168.1.101".to_string(),
            dst_ip: "8.8.8.8".to_string(),
            src_port: 12345,
            dst_port: 53,
            protocol: "UDP".to_string(),
            state: ConnState::New,
            packets_in: 1,
            packets_out: 1,
            bytes_in: 64,
            bytes_out: 40,
            last_seen: chrono::Utc::now().timestamp() as u64,
            timeout: 30,
        },
        ConntrackInfo {
            src_ip: "10.0.0.50".to_string(),
            dst_ip: "192.168.1.100".to_string(),
            src_port: 80,
            dst_port: 49876,
            protocol: "TCP".to_string(),
            state: ConnState::Related,
            packets_in: 350,
            packets_out: 200,
            bytes_in: 450_000,
            bytes_out: 120_000,
            last_seen: chrono::Utc::now().timestamp() as u64,
            timeout: 120,
        },
        ConntrackInfo {
            src_ip: "172.16.0.5".to_string(),
            dst_ip: "192.168.1.1".to_string(),
            src_port: 0,
            dst_port: 0,
            protocol: "ICMP".to_string(),
            state: ConnState::Invalid,
            packets_in: 5,
            packets_out: 0,
            bytes_in: 420,
            bytes_out: 0,
            last_seen: chrono::Utc::now().timestamp() as u64,
            timeout: 10,
        },
    ];
    Ok(Json(mock_entries))
}

// === Logs ===

async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LogQuery>,
) -> AppResult<Vec<PacketLog>> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let results = state
        .db
        .scan_prefix::<PacketLog>("log:")
        .await
        .map_err(internal_error)?;

    let mut logs: Vec<PacketLog> = results.into_iter().map(|(_, l)| l).collect();

    // If no logs exist, generate sample entries for demo purposes
    if logs.is_empty() {
        let now = chrono::Utc::now().timestamp();
        logs = vec![
            PacketLog {
                timestamp: now - 300,
                src_ip: "192.168.1.100".to_string(),
                dst_ip: "10.0.0.1".to_string(),
                src_port: 54321,
                dst_port: 443,
                protocol: "TCP".to_string(),
                action: "allow".to_string(),
                rule_id: 1,
                interface: "eth0".to_string(),
                bytes: 1500,
            },
            PacketLog {
                timestamp: now - 240,
                src_ip: "10.0.0.50".to_string(),
                dst_ip: "192.168.1.100".to_string(),
                src_port: 80,
                dst_port: 49876,
                protocol: "TCP".to_string(),
                action: "allow".to_string(),
                rule_id: 2,
                interface: "eth0".to_string(),
                bytes: 2048,
            },
            PacketLog {
                timestamp: now - 180,
                src_ip: "172.16.0.5".to_string(),
                dst_ip: "192.168.1.1".to_string(),
                src_port: 0,
                dst_port: 0,
                protocol: "ICMP".to_string(),
                action: "drop".to_string(),
                rule_id: 3,
                interface: "eth1".to_string(),
                bytes: 84,
            },
            PacketLog {
                timestamp: now - 120,
                src_ip: "192.168.1.101".to_string(),
                dst_ip: "8.8.8.8".to_string(),
                src_port: 12345,
                dst_port: 53,
                protocol: "UDP".to_string(),
                action: "allow".to_string(),
                rule_id: 1,
                interface: "eth0".to_string(),
                bytes: 64,
            },
            PacketLog {
                timestamp: now - 60,
                src_ip: "203.0.113.10".to_string(),
                dst_ip: "192.168.1.100".to_string(),
                src_port: 44444,
                dst_port: 22,
                protocol: "TCP".to_string(),
                action: "drop".to_string(),
                rule_id: 5,
                interface: "eth0".to_string(),
                bytes: 60,
            },
        ];
    }

    // Apply filters
    if let Some(ref src_ip) = params.src_ip {
        logs.retain(|l| &l.src_ip == src_ip);
    }
    if let Some(ref dst_ip) = params.dst_ip {
        logs.retain(|l| &l.dst_ip == dst_ip);
    }
    if let Some(ref protocol) = params.protocol {
        logs.retain(|l| l.protocol.eq_ignore_ascii_case(protocol));
    }
    if let Some(ref action) = params.action {
        logs.retain(|l| l.action.eq_ignore_ascii_case(action));
    }

    // Apply offset and limit
    let logs: Vec<PacketLog> = logs.into_iter().skip(offset).take(limit).collect();

    Ok(Json(logs))
}

// === Network Interfaces ===

async fn list_interfaces() -> Json<Vec<NetworkInterface>> {
    #[cfg(target_os = "linux")]
    {
        let interfaces = read_linux_interfaces().unwrap_or_else(|_| mock_interfaces());
        Json(interfaces)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Json(mock_interfaces())
    }
}

#[cfg(target_os = "linux")]
fn read_linux_interfaces() -> Result<Vec<NetworkInterface>, std::io::Error> {
    use std::fs;

    let mut interfaces = Vec::new();
    let net_dir = std::path::Path::new("/sys/class/net");

    for entry in fs::read_dir(net_dir)? {
        let entry = entry?;
        let name = entry.file_name().into_string().unwrap_or_default();

        let mac = fs::read_to_string(entry.path().join("address"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let mtu: u32 = fs::read_to_string(entry.path().join("mtu"))
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(1500);
        let operstate = fs::read_to_string(entry.path().join("operstate"))
            .unwrap_or_default()
            .trim()
            .to_string();

        interfaces.push(NetworkInterface {
            name,
            mac,
            ip: String::new(), // Would need netlink/ioctl to get IP
            status: operstate,
            mtu,
        });
    }

    Ok(interfaces)
}

fn mock_interfaces() -> Vec<NetworkInterface> {
    vec![
        NetworkInterface {
            name: "eth0".to_string(),
            mac: "00:11:22:33:44:55".to_string(),
            ip: "192.168.1.1".to_string(),
            status: "up".to_string(),
            mtu: 1500,
        },
        NetworkInterface {
            name: "eth1".to_string(),
            mac: "00:11:22:33:44:56".to_string(),
            ip: "10.0.0.1".to_string(),
            status: "up".to_string(),
            mtu: 1500,
        },
        NetworkInterface {
            name: "lo".to_string(),
            mac: "00:00:00:00:00:00".to_string(),
            ip: "127.0.0.1".to_string(),
            status: "up".to_string(),
            mtu: 65536,
        },
    ]
}

// === Backup / Restore ===

async fn backup_data(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BackupData>, (StatusCode, String)> {
    let rules = state
        .db
        .scan_prefix::<serde_json::Value>("rule:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let nat_entries = state
        .db
        .scan_prefix::<serde_json::Value>("nat:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let routes = state
        .db
        .scan_prefix::<serde_json::Value>("route:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let zones = state
        .db
        .scan_prefix::<serde_json::Value>("zone:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let policies = state
        .db
        .scan_prefix::<serde_json::Value>("policy:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let backup = BackupData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        rules,
        nat_entries,
        routes,
        zones,
        policies,
    };

    Ok(Json(backup))
}

async fn restore_data(
    State(state): State<Arc<AppState>>,
    Json(backup): Json<BackupData>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    // Clear existing data for each prefix
    for prefix in &["rule:", "nat:", "route:", "zone:", "policy:"] {
        let existing = state
            .db
            .scan_prefix::<serde_json::Value>(prefix)
            .await
            .map_err(internal_error)?;
        for (key, _) in &existing {
            state.db.delete(key).await.map_err(internal_error)?;
        }
        // Clear the index
        let index_key = format!("{}__index", prefix);
        let empty: Vec<String> = Vec::new();
        state
            .db
            .put(&index_key, &empty)
            .await
            .map_err(internal_error)?;
    }

    // Restore rules
    for rule in &backup.rules {
        let id = rule.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let key = format!("rule:{}", id);
        state.db.put(&key, rule).await.map_err(internal_error)?;
        state
            .db
            .add_to_index("rule:", &key)
            .await
            .map_err(internal_error)?;
    }

    // Restore NAT entries
    for entry in &backup.nat_entries {
        let id = entry.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let key = format!("nat:{}", id);
        state.db.put(&key, entry).await.map_err(internal_error)?;
        state
            .db
            .add_to_index("nat:", &key)
            .await
            .map_err(internal_error)?;
    }

    // Restore routes
    for route in &backup.routes {
        let id = route.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let key = format!("route:{}", id);
        state.db.put(&key, route).await.map_err(internal_error)?;
        state
            .db
            .add_to_index("route:", &key)
            .await
            .map_err(internal_error)?;
    }

    // Restore zones
    for zone in &backup.zones {
        let id = zone.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let key = format!("zone:{}", id);
        state.db.put(&key, zone).await.map_err(internal_error)?;
        state
            .db
            .add_to_index("zone:", &key)
            .await
            .map_err(internal_error)?;
    }

    // Restore policies
    for policy in &backup.policies {
        let id = policy.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let key = format!("policy:{}", id);
        state.db.put(&key, policy).await.map_err(internal_error)?;
        state
            .db
            .add_to_index("policy:", &key)
            .await
            .map_err(internal_error)?;
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "restored",
            "rules": backup.rules.len(),
            "nat_entries": backup.nat_entries.len(),
            "routes": backup.routes.len(),
            "zones": backup.zones.len(),
            "policies": backup.policies.len(),
        })),
    ))
}
