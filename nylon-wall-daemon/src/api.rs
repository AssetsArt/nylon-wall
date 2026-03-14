use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use futures_util::{SinkExt, StreamExt};
use nylon_wall_common::conntrack::ConntrackInfo;
use nylon_wall_common::dhcp::{
    DhcpClientConfig, DhcpClientStatus, DhcpLease, DhcpLeaseState, DhcpPool, DhcpReservation,
};
use nylon_wall_common::log::PacketLog;
use nylon_wall_common::nat::NatEntry;
#[cfg(target_os = "linux")]
use nylon_wall_common::nat::NatType;
use nylon_wall_common::route::{PolicyRoute, Route};
use nylon_wall_common::rule::FirewallRule;
use nylon_wall_common::tls::{SniRule, SniStats};
use nylon_wall_common::zone::{NetworkPolicy, Zone};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::AppState;
use crate::changeset;
use crate::events::WsEvent;

type AppResult<T> = Result<Json<T>, (StatusCode, String)>;

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// Helper to broadcast an event (best-effort, ignores errors if no subscribers).
fn broadcast(state: &AppState, event: WsEvent) {
    let _ = state.event_tx.send(event);
}

/// Serialize a value to JSON, falling back to `null` instead of panicking.
fn to_json_value(v: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

// === Change Management ===

#[derive(Serialize)]
struct PendingChangeStatus {
    pending: bool,
    description: String,
    remaining_secs: u64,
    total_secs: u64,
}

async fn changes_pending(State(state): State<Arc<AppState>>) -> Json<PendingChangeStatus> {
    let total = changeset::revert_timeout_secs();
    match changeset::status(&state.pending_changes).await {
        Some((desc, remaining)) => Json(PendingChangeStatus {
            pending: true,
            description: desc,
            remaining_secs: remaining,
            total_secs: total,
        }),
        None => Json(PendingChangeStatus {
            pending: false,
            description: String::new(),
            remaining_secs: 0,
            total_secs: total,
        }),
    }
}

async fn changes_confirm(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let confirmed = changeset::confirm(&state).await;
    tracing::info!("Confirmed pending change");
    Json(serde_json::json!({ "confirmed": confirmed }))
}

async fn changes_revert(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let reverted = changeset::rollback(&state).await.map_err(internal_error)?;
    tracing::warn!("Manually reverted pending change");
    // Re-sync eBPF after revert
    sync_rules_to_ebpf(&state).await;
    sync_nat_to_ebpf(&state).await;
    sync_zones_to_ebpf(&state).await;
    sync_sni_to_ebpf(&state).await;
    broadcast(&state, WsEvent::ChangesReverted { count: if reverted { 1 } else { 0 } });
    Ok(Json(serde_json::json!({ "reverted": reverted })))
}

/// Check if there's already a pending change. If so, reject the mutation.
async fn require_no_pending(state: &AppState) -> Result<(), (StatusCode, String)> {
    if changeset::has_pending(&state.pending_changes).await {
        return Err((
            StatusCode::CONFLICT,
            "A change is pending confirmation. Confirm or revert before making another change."
                .to_string(),
        ));
    }
    Ok(())
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
    #[serde(default)]
    dhcp_pools: Vec<serde_json::Value>,
    #[serde(default)]
    dhcp_reservations: Vec<serde_json::Value>,
    #[serde(default)]
    dhcp_clients: Vec<serde_json::Value>,
}

pub async fn serve(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        // WebSocket
        .route("/api/v1/ws/events", get(ws_handler))
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
        .route("/api/v1/nat/{id}/toggle", post(toggle_nat))
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
        .route("/api/v1/zones/{id}", put(update_zone).delete(delete_zone))
        // Policies
        .route("/api/v1/policies", get(list_policies).post(create_policy))
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
        .route("/api/v1/system/apply", post(apply_config))
        .route("/api/v1/system/backup", post(backup_data))
        .route("/api/v1/system/restore", post(restore_data))
        // DHCP Server — Pools
        .route(
            "/api/v1/dhcp/pools",
            get(list_dhcp_pools).post(create_dhcp_pool),
        )
        .route(
            "/api/v1/dhcp/pools/{id}",
            get(get_dhcp_pool)
                .put(update_dhcp_pool)
                .delete(delete_dhcp_pool),
        )
        .route("/api/v1/dhcp/pools/{id}/toggle", post(toggle_dhcp_pool))
        // DHCP Server — Leases
        .route("/api/v1/dhcp/leases", get(list_dhcp_leases))
        .route(
            "/api/v1/dhcp/leases/{mac}",
            axum::routing::delete(delete_dhcp_lease),
        )
        .route(
            "/api/v1/dhcp/leases/{mac}/reserve",
            post(reserve_dhcp_lease),
        )
        // DHCP Server — Reservations
        .route(
            "/api/v1/dhcp/reservations",
            get(list_dhcp_reservations).post(create_dhcp_reservation),
        )
        .route(
            "/api/v1/dhcp/reservations/{id}",
            put(update_dhcp_reservation).delete(delete_dhcp_reservation),
        )
        // DHCP Client
        .route(
            "/api/v1/dhcp/clients",
            get(list_dhcp_clients).post(create_dhcp_client),
        )
        .route(
            "/api/v1/dhcp/clients/{id}",
            put(update_dhcp_client).delete(delete_dhcp_client),
        )
        .route("/api/v1/dhcp/clients/{id}/toggle", post(toggle_dhcp_client))
        .route("/api/v1/dhcp/clients/status", get(list_dhcp_client_status))
        .route(
            "/api/v1/dhcp/clients/{interface}/release",
            post(release_dhcp_client),
        )
        .route(
            "/api/v1/dhcp/clients/{interface}/renew",
            post(renew_dhcp_client),
        )
        // SNI Filtering
        .route(
            "/api/v1/tls/sni/rules",
            get(list_sni_rules).post(create_sni_rule),
        )
        .route(
            "/api/v1/tls/sni/rules/{id}",
            put(update_sni_rule).delete(delete_sni_rule),
        )
        .route("/api/v1/tls/sni/rules/{id}/toggle", post(toggle_sni_rule))
        .route("/api/v1/tls/sni/stats", get(sni_stats))
        .route("/api/v1/tls/sni/toggle", post(toggle_sni_filtering))
        .route("/api/v1/tls/sni/debug", get(debug_sni_maps))
        // Change management (auto-revert)
        .route("/api/v1/changes/pending", get(changes_pending))
        .route("/api/v1/changes/confirm", post(changes_confirm))
        .route("/api/v1/changes/revert", post(changes_revert))
        // Prometheus metrics
        .route("/metrics", get(prometheus_metrics))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

// === WebSocket ===

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = state.event_tx.subscribe();

    // Forward broadcast events to this WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(json) => json,
                Err(_) => continue,
            };
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Consume incoming messages (ping/pong handled by axum, we just drain)
    let recv_task = tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    // When either task finishes, abort the other
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
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
    Json(mut rule): Json<FirewallRule>,
) -> Result<(StatusCode, Json<FirewallRule>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<FirewallRule>("rule:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, r)| r.id).max().unwrap_or(0) + 1;
    rule.id = next_id;
    let key = format!("rule:{}", rule.id);
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("rule:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "rule:", &key, format!("Created rule '{}'", rule.name)).await;
    broadcast(
        &state,
        WsEvent::RuleCreated(to_json_value(&rule)),
    );
    sync_rules_to_ebpf(&state).await;
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
    require_no_pending(&state).await?;
    rule.id = id;
    let key = format!("rule:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated rule #{}", id)).await;
    }
    state
        .db
        .add_to_index("rule:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::RuleUpdated(to_json_value(&rule)),
    );
    sync_rules_to_ebpf(&state).await;
    Ok(Json(rule))
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("rule:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("rule:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "rule:", &key, old_val, format!("Deleted rule #{}", id)).await;
    }
    broadcast(&state, WsEvent::RuleDeleted { id });
    sync_rules_to_ebpf(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<FirewallRule> {
    require_no_pending(&state).await?;
    let key = format!("rule:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    let mut rule: FirewallRule = state
        .db
        .get(&key)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Rule not found".to_string()))?;
    rule.enabled = !rule.enabled;
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Toggled rule #{}", id)).await;
    }
    broadcast(
        &state,
        WsEvent::RuleToggled(to_json_value(&rule)),
    );
    sync_rules_to_ebpf(&state).await;
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
    Json(mut entry): Json<NatEntry>,
) -> Result<(StatusCode, Json<NatEntry>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<NatEntry>("nat:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, e)| e.id).max().unwrap_or(0) + 1;
    entry.id = next_id;
    let key = format!("nat:{}", entry.id);
    state.db.put(&key, &entry).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("nat:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "nat:", &key, format!("Created NAT entry #{}", entry.id)).await;
    broadcast(
        &state,
        WsEvent::NatCreated(to_json_value(&entry)),
    );
    sync_nat_to_ebpf(&state).await;
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn update_nat(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut entry): Json<NatEntry>,
) -> AppResult<NatEntry> {
    require_no_pending(&state).await?;
    entry.id = id;
    let key = format!("nat:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &entry).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated NAT entry #{}", id)).await;
    }
    state
        .db
        .add_to_index("nat:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::NatUpdated(to_json_value(&entry)),
    );
    sync_nat_to_ebpf(&state).await;
    Ok(Json(entry))
}

async fn delete_nat(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("nat:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("nat:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "nat:", &key, old_val, format!("Deleted NAT entry #{}", id)).await;
    }
    broadcast(&state, WsEvent::NatDeleted { id });
    sync_nat_to_ebpf(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_nat(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<NatEntry> {
    require_no_pending(&state).await?;
    let key = format!("nat:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    let mut entry: NatEntry = state
        .db
        .get(&key)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "NAT entry not found".to_string()))?;
    entry.enabled = !entry.enabled;
    state.db.put(&key, &entry).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Toggled NAT entry #{}", id)).await;
    }
    broadcast(
        &state,
        WsEvent::NatToggled(to_json_value(&entry)),
    );
    sync_nat_to_ebpf(&state).await;
    Ok(Json(entry))
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
    Json(mut route): Json<Route>,
) -> Result<(StatusCode, Json<Route>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<Route>("route:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, r)| r.id).max().unwrap_or(0) + 1;
    route.id = next_id;
    let key = format!("route:{}", route.id);
    state.db.put(&key, &route).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("route:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "route:", &key, format!("Created route #{}", route.id)).await;
    broadcast(
        &state,
        WsEvent::RouteCreated(to_json_value(&route)),
    );
    Ok((StatusCode::CREATED, Json(route)))
}

async fn update_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut route): Json<Route>,
) -> AppResult<Route> {
    require_no_pending(&state).await?;
    route.id = id;
    let key = format!("route:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &route).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated route #{}", id)).await;
    }
    state
        .db
        .add_to_index("route:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::RouteUpdated(to_json_value(&route)),
    );
    Ok(Json(route))
}

async fn delete_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("route:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("route:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "route:", &key, old_val, format!("Deleted route #{}", id)).await;
    }
    broadcast(&state, WsEvent::RouteDeleted { id });
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
    Json(mut route): Json<PolicyRoute>,
) -> Result<(StatusCode, Json<PolicyRoute>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<PolicyRoute>("policy_route:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, r)| r.id).max().unwrap_or(0) + 1;
    route.id = next_id;
    let key = format!("policy_route:{}", route.id);
    state.db.put(&key, &route).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("policy_route:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "policy_route:", &key, format!("Created policy route #{}", route.id)).await;
    Ok((StatusCode::CREATED, Json(route)))
}

async fn update_policy_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut route): Json<PolicyRoute>,
) -> AppResult<PolicyRoute> {
    require_no_pending(&state).await?;
    route.id = id;
    let key = format!("policy_route:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &route).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated policy route #{}", id)).await;
    }
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
    require_no_pending(&state).await?;
    let key = format!("policy_route:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("policy_route:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "policy_route:", &key, old_val, format!("Deleted policy route #{}", id)).await;
    }
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
    Json(mut zone): Json<Zone>,
) -> Result<(StatusCode, Json<Zone>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<Zone>("zone:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, z)| z.id).max().unwrap_or(0) + 1;
    zone.id = next_id;
    let key = format!("zone:{}", zone.id);
    state.db.put(&key, &zone).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("zone:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "zone:", &key, format!("Created zone '{}'", zone.name)).await;
    broadcast(
        &state,
        WsEvent::ZoneCreated(to_json_value(&zone)),
    );
    Ok((StatusCode::CREATED, Json(zone)))
}

async fn update_zone(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut zone): Json<Zone>,
) -> AppResult<Zone> {
    require_no_pending(&state).await?;
    zone.id = id;
    let key = format!("zone:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &zone).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated zone #{}", id)).await;
    }
    state
        .db
        .add_to_index("zone:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::ZoneUpdated(to_json_value(&zone)),
    );
    Ok(Json(zone))
}

async fn delete_zone(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("zone:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("zone:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "zone:", &key, old_val, format!("Deleted zone #{}", id)).await;
    }
    broadcast(&state, WsEvent::ZoneDeleted { id });
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
    Json(mut policy): Json<NetworkPolicy>,
) -> Result<(StatusCode, Json<NetworkPolicy>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<NetworkPolicy>("policy:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, p)| p.id).max().unwrap_or(0) + 1;
    policy.id = next_id;
    let key = format!("policy:{}", policy.id);
    state.db.put(&key, &policy).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("policy:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "policy:", &key, format!("Created policy '{}'", policy.name)).await;
    broadcast(
        &state,
        WsEvent::PolicyCreated(to_json_value(&policy)),
    );
    Ok((StatusCode::CREATED, Json(policy)))
}

async fn update_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut policy): Json<NetworkPolicy>,
) -> AppResult<NetworkPolicy> {
    require_no_pending(&state).await?;
    policy.id = id;
    let key = format!("policy:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &policy).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated policy #{}", id)).await;
    }
    state
        .db
        .add_to_index("policy:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::PolicyUpdated(to_json_value(&policy)),
    );
    Ok(Json(policy))
}

async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("policy:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("policy:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "policy:", &key, old_val, format!("Deleted policy #{}", id)).await;
    }
    broadcast(&state, WsEvent::PolicyDeleted { id });
    Ok(StatusCode::NO_CONTENT)
}

// === System ===

#[derive(serde::Serialize)]
struct SystemStatus {
    version: String,
    ebpf_loaded: bool,
    uptime_seconds: u64,
}

async fn system_status(State(state): State<Arc<AppState>>) -> Json<SystemStatus> {
    Json(SystemStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        ebpf_loaded: state.ebpf_loaded.load(std::sync::atomic::Ordering::Relaxed),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

async fn prometheus_metrics(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, [(String, String); 1], String) {
    let body = crate::metrics::collect(&state).await;
    (
        StatusCode::OK,
        [(
            "content-type".to_string(),
            "text/plain; version=0.0.4; charset=utf-8".to_string(),
        )],
        body,
    )
}

// === Rules Reorder ===

async fn reorder_rules(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReorderRequest>,
) -> Result<Json<Vec<FirewallRule>>, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let mut updated_rules = Vec::new();
    for (index, rule_id) in req.rule_ids.iter().enumerate() {
        let key = format!("rule:{}", rule_id);
        let mut rule: FirewallRule = state.db.get(&key).await.map_err(internal_error)?.ok_or((
            StatusCode::NOT_FOUND,
            format!("Rule not found: {}", rule_id),
        ))?;
        rule.priority = (index * 10) as u32;
        state.db.put(&key, &rule).await.map_err(internal_error)?;
        updated_rules.push(rule);
    }
    sync_rules_to_ebpf(&state).await;
    Ok(Json(updated_rules))
}

// === Conntrack ===

#[derive(Deserialize)]
struct ConntrackQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    state: Option<String>,
    protocol: Option<String>,
}

#[derive(Serialize)]
struct PaginatedConntrack {
    total: usize,
    offset: usize,
    limit: usize,
    entries: Vec<ConntrackInfo>,
}

async fn list_conntrack(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ConntrackQuery>,
) -> AppResult<PaginatedConntrack> {
    let mut entries = crate::state::get_connections(&state).await;

    // Filter by state
    if let Some(ref s) = params.state {
        entries.retain(|e| format!("{}", e.state).eq_ignore_ascii_case(s));
    }
    // Filter by protocol
    if let Some(ref p) = params.protocol {
        entries.retain(|e| e.protocol.eq_ignore_ascii_case(p));
    }

    let total = entries.len();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50);

    let page: Vec<ConntrackInfo> = entries.into_iter().skip(offset).take(limit).collect();

    Ok(Json(PaginatedConntrack {
        total,
        offset,
        limit,
        entries: page,
    }))
}

// === Logs ===

async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LogQuery>,
) -> AppResult<serde_json::Value> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let results = state
        .db
        .scan_prefix::<PacketLog>("log:")
        .await
        .map_err(internal_error)?;

    let mut logs: Vec<PacketLog> = results.into_iter().map(|(_, l)| l).collect();

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

    let total = logs.len();

    // Apply offset and limit
    let entries: Vec<PacketLog> = logs.into_iter().skip(offset).take(limit).collect();

    Ok(Json(serde_json::json!({
        "total": total,
        "offset": offset,
        "limit": limit,
        "entries": entries,
    })))
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

// === DHCP Pools ===

async fn list_dhcp_pools(State(state): State<Arc<AppState>>) -> AppResult<Vec<DhcpPool>> {
    let results = state
        .db
        .scan_prefix::<DhcpPool>("dhcp_pool:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, p)| p).collect()))
}

async fn get_dhcp_pool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<DhcpPool> {
    let key = format!("dhcp_pool:{}", id);
    match state
        .db
        .get::<DhcpPool>(&key)
        .await
        .map_err(internal_error)?
    {
        Some(pool) => Ok(Json(pool)),
        None => Err((StatusCode::NOT_FOUND, "DHCP pool not found".to_string())),
    }
}

async fn create_dhcp_pool(
    State(state): State<Arc<AppState>>,
    Json(mut pool): Json<DhcpPool>,
) -> Result<(StatusCode, Json<DhcpPool>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<DhcpPool>("dhcp_pool:").await.map_err(internal_error)?;
    // Prevent duplicate interface
    if existing.iter().any(|(_, p)| p.interface == pool.interface) {
        return Err((
            StatusCode::CONFLICT,
            format!("A DHCP pool already exists for interface '{}'", pool.interface),
        ));
    }
    let next_id = existing.iter().map(|(_, p)| p.id).max().unwrap_or(0) + 1;
    pool.id = next_id;
    let key = format!("dhcp_pool:{}", pool.id);
    state.db.put(&key, &pool).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("dhcp_pool:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "dhcp_pool:", &key, format!("Created DHCP pool #{}", pool.id)).await;
    broadcast(
        &state,
        WsEvent::DhcpPoolCreated(to_json_value(&pool)),
    );
    // Notify DHCP server to reload pools
    let _ = state.dhcp_pool_notify.send(());
    Ok((StatusCode::CREATED, Json(pool)))
}

async fn update_dhcp_pool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut pool): Json<DhcpPool>,
) -> AppResult<DhcpPool> {
    require_no_pending(&state).await?;
    pool.id = id;
    // Prevent duplicate interface (exclude self)
    let existing = state.db.scan_prefix::<DhcpPool>("dhcp_pool:").await.map_err(internal_error)?;
    if existing.iter().any(|(_, p)| p.interface == pool.interface && p.id != id) {
        return Err((
            StatusCode::CONFLICT,
            format!("A DHCP pool already exists for interface '{}'", pool.interface),
        ));
    }
    let key = format!("dhcp_pool:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &pool).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated DHCP pool #{}", id)).await;
    }
    state
        .db
        .add_to_index("dhcp_pool:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::DhcpPoolUpdated(to_json_value(&pool)),
    );
    let _ = state.dhcp_pool_notify.send(());
    Ok(Json(pool))
}

async fn delete_dhcp_pool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("dhcp_pool:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("dhcp_pool:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "dhcp_pool:", &key, old_val, format!("Deleted DHCP pool #{}", id)).await;
    }
    broadcast(&state, WsEvent::DhcpPoolDeleted { id });
    let _ = state.dhcp_pool_notify.send(());
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_dhcp_pool(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<DhcpPool> {
    require_no_pending(&state).await?;
    let key = format!("dhcp_pool:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    let mut pool: DhcpPool = state
        .db
        .get(&key)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "DHCP pool not found".to_string()))?;
    pool.enabled = !pool.enabled;
    state.db.put(&key, &pool).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Toggled DHCP pool #{}", id)).await;
    }
    broadcast(
        &state,
        WsEvent::DhcpPoolUpdated(to_json_value(&pool)),
    );
    let _ = state.dhcp_pool_notify.send(());
    Ok(Json(pool))
}

// === DHCP Leases ===

async fn list_dhcp_leases(State(state): State<Arc<AppState>>) -> AppResult<Vec<DhcpLease>> {
    let results = state
        .db
        .scan_prefix::<DhcpLease>("dhcp_lease:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, l)| l).collect()))
}

async fn delete_dhcp_lease(
    State(state): State<Arc<AppState>>,
    Path(mac): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let key = format!("dhcp_lease:{}", mac);
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("dhcp_lease:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(
        &state,
        WsEvent::DhcpLeaseChanged(serde_json::json!({"mac": mac, "action": "released"})),
    );
    Ok(StatusCode::NO_CONTENT)
}

async fn reserve_dhcp_lease(
    State(state): State<Arc<AppState>>,
    Path(mac): Path<String>,
) -> Result<(StatusCode, Json<DhcpReservation>), (StatusCode, String)> {
    // Look up the current lease
    let lease_key = format!("dhcp_lease:{}", mac);
    let lease: DhcpLease = state
        .db
        .get(&lease_key)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Lease not found".to_string()))?;

    // Generate a new reservation ID
    let existing = state
        .db
        .scan_prefix::<DhcpReservation>("dhcp_reservation:")
        .await
        .map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, r)| r.id).max().unwrap_or(0) + 1;

    let reservation = DhcpReservation {
        id: next_id,
        pool_id: lease.pool_id,
        mac: mac.clone(),
        ip: lease.ip.clone(),
        hostname: lease.hostname.clone(),
    };

    let key = format!("dhcp_reservation:{}", next_id);
    state
        .db
        .put(&key, &reservation)
        .await
        .map_err(internal_error)?;
    state
        .db
        .add_to_index("dhcp_reservation:", &key)
        .await
        .map_err(internal_error)?;

    // Update the lease state to Reserved
    let mut updated_lease = lease;
    updated_lease.state = DhcpLeaseState::Reserved;
    state
        .db
        .put(&lease_key, &updated_lease)
        .await
        .map_err(internal_error)?;

    broadcast(
        &state,
        WsEvent::DhcpReservationCreated(to_json_value(&reservation)),
    );
    Ok((StatusCode::CREATED, Json(reservation)))
}

// === DHCP Reservations ===

async fn list_dhcp_reservations(
    State(state): State<Arc<AppState>>,
) -> AppResult<Vec<DhcpReservation>> {
    let results = state
        .db
        .scan_prefix::<DhcpReservation>("dhcp_reservation:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, r)| r).collect()))
}

async fn create_dhcp_reservation(
    State(state): State<Arc<AppState>>,
    Json(mut reservation): Json<DhcpReservation>,
) -> Result<(StatusCode, Json<DhcpReservation>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<DhcpReservation>("dhcp_reservation:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, r)| r.id).max().unwrap_or(0) + 1;
    reservation.id = next_id;
    let key = format!("dhcp_reservation:{}", reservation.id);
    state
        .db
        .put(&key, &reservation)
        .await
        .map_err(internal_error)?;
    state
        .db
        .add_to_index("dhcp_reservation:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(&state, "dhcp_reservation:", &key, format!("Created DHCP reservation #{}", reservation.id)).await;
    broadcast(
        &state,
        WsEvent::DhcpReservationCreated(to_json_value(&reservation)),
    );
    Ok((StatusCode::CREATED, Json(reservation)))
}

async fn update_dhcp_reservation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut reservation): Json<DhcpReservation>,
) -> AppResult<DhcpReservation> {
    require_no_pending(&state).await?;
    reservation.id = id;
    let key = format!("dhcp_reservation:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state
        .db
        .put(&key, &reservation)
        .await
        .map_err(internal_error)?;
    state
        .db
        .add_to_index("dhcp_reservation:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated DHCP reservation #{}", id)).await;
    }
    Ok(Json(reservation))
}

async fn delete_dhcp_reservation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("dhcp_reservation:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("dhcp_reservation:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "dhcp_reservation:", &key, old_val, format!("Deleted DHCP reservation #{}", id)).await;
    }
    broadcast(&state, WsEvent::DhcpReservationDeleted { id });
    Ok(StatusCode::NO_CONTENT)
}

// === DHCP Client ===

async fn list_dhcp_clients(State(state): State<Arc<AppState>>) -> AppResult<Vec<DhcpClientConfig>> {
    let results = state
        .db
        .scan_prefix::<DhcpClientConfig>("dhcp_client:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, c)| c).collect()))
}

async fn create_dhcp_client(
    State(state): State<Arc<AppState>>,
    Json(mut config): Json<DhcpClientConfig>,
) -> Result<(StatusCode, Json<DhcpClientConfig>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state.db.scan_prefix::<DhcpClientConfig>("dhcp_client:").await.map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, c)| c.id).max().unwrap_or(0) + 1;
    config.id = next_id;
    let key = format!("dhcp_client:{}", config.id);
    state.db.put(&key, &config).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("dhcp_client:", &key)
        .await
        .map_err(internal_error)?;

    changeset::record_create(&state, "dhcp_client:", &key, format!("Created DHCP client #{}", config.id)).await;

    // If enabled, spawn a client task
    if config.enabled {
        let client_state = Arc::clone(&state);
        let client_config = config.clone();
        tokio::spawn(async move {
            crate::dhcp::client::run_dhcp_client(client_state, client_config).await;
        });
    }

    Ok((StatusCode::CREATED, Json(config)))
}

async fn update_dhcp_client(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut config): Json<DhcpClientConfig>,
) -> AppResult<DhcpClientConfig> {
    require_no_pending(&state).await?;
    config.id = id;
    let key = format!("dhcp_client:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.put(&key, &config).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Updated DHCP client #{}", id)).await;
    }
    state
        .db
        .add_to_index("dhcp_client:", &key)
        .await
        .map_err(internal_error)?;
    Ok(Json(config))
}

async fn delete_dhcp_client(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("dhcp_client:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("dhcp_client:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(&state, "dhcp_client:", &key, old_val, format!("Deleted DHCP client #{}", id)).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_dhcp_client(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<DhcpClientConfig> {
    require_no_pending(&state).await?;
    let key = format!("dhcp_client:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    let mut config: DhcpClientConfig =
        state.db.get(&key).await.map_err(internal_error)?.ok_or((
            StatusCode::NOT_FOUND,
            "DHCP client config not found".to_string(),
        ))?;
    config.enabled = !config.enabled;
    state.db.put(&key, &config).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(&state, &key, old_val, format!("Toggled DHCP client #{}", id)).await;
    }

    // If enabling, spawn client task
    if config.enabled {
        let client_state = Arc::clone(&state);
        let client_config = config.clone();
        tokio::spawn(async move {
            crate::dhcp::client::run_dhcp_client(client_state, client_config).await;
        });
    }

    Ok(Json(config))
}

async fn list_dhcp_client_status(
    State(state): State<Arc<AppState>>,
) -> AppResult<Vec<DhcpClientStatus>> {
    let results = state
        .db
        .scan_prefix::<DhcpClientStatus>("dhcp_client_status:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, s)| s).collect()))
}

async fn release_dhcp_client(
    State(state): State<Arc<AppState>>,
    Path(interface): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Update status to Idle
    let status_key = format!("dhcp_client_status:{}", interface);
    if let Some(mut status) = state
        .db
        .get::<DhcpClientStatus>(&status_key)
        .await
        .map_err(internal_error)?
    {
        status.state = nylon_wall_common::dhcp::DhcpClientState::Idle;
        status.ip = None;
        status.subnet_mask = None;
        status.gateway = None;
        status.dns_servers = Vec::new();
        state
            .db
            .put(&status_key, &status)
            .await
            .map_err(internal_error)?;
        broadcast(
            &state,
            WsEvent::DhcpClientStatusChanged(to_json_value(&status)),
        );
    }
    Ok(Json(
        serde_json::json!({"status": "released", "interface": interface}),
    ))
}

async fn renew_dhcp_client(
    State(state): State<Arc<AppState>>,
    Path(interface): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Find the client config for this interface
    let configs = state
        .db
        .scan_prefix::<DhcpClientConfig>("dhcp_client:")
        .await
        .map_err(internal_error)?;

    let config = configs
        .into_iter()
        .map(|(_, c)| c)
        .find(|c| c.interface == interface)
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("No DHCP client config for interface {}", interface),
        ))?;

    // Spawn a new client task to renew
    let client_state = Arc::clone(&state);
    tokio::spawn(async move {
        crate::dhcp::client::run_dhcp_client(client_state, config).await;
    });

    Ok(Json(
        serde_json::json!({"status": "renewing", "interface": interface}),
    ))
}

// === eBPF Auto-Sync ===

/// Read all rules from DB, compile them, and push to eBPF maps.
/// Called automatically after every rule mutation.
pub async fn sync_rules_to_ebpf(state: &AppState) {
    let rules = match state.db.scan_prefix::<FirewallRule>("rule:").await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to read rules for eBPF sync: {}", e);
            return;
        }
    };

    let mut ingress_rules: Vec<FirewallRule> = Vec::new();
    let mut egress_rules: Vec<FirewallRule> = Vec::new();

    for (_, rule) in &rules {
        if rule.enabled {
            match rule.direction {
                nylon_wall_common::rule::Direction::Ingress => ingress_rules.push(rule.clone()),
                nylon_wall_common::rule::Direction::Egress => egress_rules.push(rule.clone()),
            }
        }
    }

    ingress_rules.sort_by_key(|r| r.priority);
    egress_rules.sort_by_key(|r| r.priority);

    #[cfg(target_os = "linux")]
    {
        use nylon_wall_common::rule::EbpfRule;
        let ebpf_ingress: Vec<EbpfRule> = ingress_rules
            .iter()
            .map(|r| crate::ebpf_loader::firewall_rule_to_ebpf(r))
            .collect();
        let ebpf_egress: Vec<EbpfRule> = egress_rules
            .iter()
            .map(|r| crate::ebpf_loader::firewall_rule_to_ebpf(r))
            .collect();

        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            match crate::ebpf_loader::sync_rules_to_maps(bpf, &ebpf_ingress, &ebpf_egress) {
                Ok(_) => tracing::debug!(
                    "Auto-synced {} ingress + {} egress rules to eBPF",
                    ebpf_ingress.len(),
                    ebpf_egress.len()
                ),
                Err(e) => tracing::error!("Failed to sync eBPF maps: {}", e),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::debug!(
            "eBPF sync skipped (not Linux): {} ingress + {} egress rules",
            ingress_rules.len(),
            egress_rules.len()
        );
    }
}

/// Sync NAT entries from DB to eBPF maps.
pub async fn sync_nat_to_ebpf(state: &AppState) {
    let entries = match state.db.scan_prefix::<NatEntry>("nat:").await {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to read NAT entries: {}", e);
            return;
        }
    };

    #[cfg(target_os = "linux")]
    {
        let enabled_entries: Vec<_> = entries
            .iter()
            .filter(|(_, e)| e.enabled)
            .collect();

        let ebpf_entries: Vec<_> = enabled_entries
            .iter()
            .map(|(_, e)| crate::nat::nat_entry_to_ebpf(e))
            .collect();

        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            if let Err(e) = crate::ebpf_loader::sync_nat_to_maps(bpf, &ebpf_entries) {
                tracing::error!("Failed to sync NAT to eBPF: {}", e);
            }
        } else {
            tracing::debug!("NAT sync: eBPF not loaded ({} entries skipped)", ebpf_entries.len());
        }

        // Enable route_localnet on interfaces that have DNAT rules targeting
        // loopback addresses (127.0.0.0/8), otherwise the kernel drops the
        // rewritten packets as martians.
        let default_iface = std::env::var("NYLON_WALL_IFACE").unwrap_or_else(|_| "eth0".to_string());
        let localnet_ifaces: Vec<String> = enabled_entries
            .iter()
            .filter(|(_, e)| {
                e.nat_type == NatType::DNAT
                    && e.translate_ip
                        .as_deref()
                        .map(|ip| ip.starts_with("127."))
                        .unwrap_or(false)
            })
            .map(|(_, e)| {
                e.in_interface.clone().unwrap_or_else(|| default_iface.clone())
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !localnet_ifaces.is_empty() {
            crate::ebpf_loader::ensure_route_localnet(&localnet_ifaces);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::debug!("NAT sync: {} entries (eBPF not available on this platform)", entries.len());
    }
}

/// Sync zone/policy mappings from DB to eBPF maps.
pub async fn sync_zones_to_ebpf(state: &AppState) {
    let zones = match state.db.scan_prefix::<Zone>("zone:").await {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("Failed to read zones for eBPF sync: {}", e);
            return;
        }
    };

    let policies = match state.db.scan_prefix::<NetworkPolicy>("policy:").await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to read policies for eBPF sync: {}", e);
            return;
        }
    };

    #[cfg(target_os = "linux")]
    {
        // Build zone mappings: resolve interface names to ifindexes
        let mut zone_mappings: Vec<(u32, u32)> = Vec::new();
        for (_, zone) in &zones {
            for iface_name in &zone.interfaces {
                // Get ifindex from interface name
                if let Ok(idx) = get_ifindex(iface_name) {
                    zone_mappings.push((idx, zone.id));
                }
            }
        }

        // Build policy mappings
        let mut policy_mappings: Vec<(u32, nylon_wall_common::zone::EbpfPolicyValue)> = Vec::new();
        for (_, policy) in &policies {
            if !policy.enabled {
                continue;
            }
            // Find zone IDs by name
            let from_zone_id = zones.iter().find(|(_, z)| z.name == policy.from_zone).map(|(_, z)| z.id);
            let to_zone_id = zones.iter().find(|(_, z)| z.name == policy.to_zone).map(|(_, z)| z.id);

            if let (Some(from_id), Some(to_id)) = (from_zone_id, to_zone_id) {
                let key = (from_id << 16) | to_id;
                let value = nylon_wall_common::zone::EbpfPolicyValue {
                    action: policy.action as u8,
                    log: if policy.log { 1 } else { 0 },
                    _pad: [0; 2],
                };
                policy_mappings.push((key, value));
            }
        }

        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            if let Err(e) = crate::ebpf_loader::sync_zones_to_maps(bpf, &zone_mappings) {
                tracing::error!("Failed to sync zone maps: {}", e);
            }
            if let Err(e) = crate::ebpf_loader::sync_policies_to_maps(bpf, &policy_mappings) {
                tracing::error!("Failed to sync policy maps: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::debug!(
            "Zone/policy eBPF sync skipped (not Linux): {} zones, {} policies",
            zones.len(),
            policies.len()
        );
    }
}

/// Get network interface index by name.
#[cfg(target_os = "linux")]
fn get_ifindex(name: &str) -> anyhow::Result<u32> {
    use std::ffi::CString;
    let cname = CString::new(name)?;
    let idx = unsafe { libc::if_nametoindex(cname.as_ptr()) };
    if idx == 0 {
        anyhow::bail!("Interface {} not found", name);
    }
    Ok(idx)
}

// === Apply Configuration ===

async fn apply_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Sync rules to eBPF
    sync_rules_to_ebpf(&state).await;

    // Gather stats
    let rules = state
        .db
        .scan_prefix::<FirewallRule>("rule:")
        .await
        .map_err(internal_error)?;

    let ingress_count = rules
        .iter()
        .filter(|(_, r)| {
            r.enabled && matches!(r.direction, nylon_wall_common::rule::Direction::Ingress)
        })
        .count();
    let egress_count = rules
        .iter()
        .filter(|(_, r)| {
            r.enabled && matches!(r.direction, nylon_wall_common::rule::Direction::Egress)
        })
        .count();

    let nat_count = state
        .db
        .scan_prefix::<serde_json::Value>("nat:")
        .await
        .map_err(internal_error)?
        .len();
    let route_count = state
        .db
        .scan_prefix::<serde_json::Value>("route:")
        .await
        .map_err(internal_error)?
        .len();

    broadcast(
        &state,
        WsEvent::RuleUpdated(serde_json::json!({"action": "config_applied"})),
    );

    Ok(Json(serde_json::json!({
        "status": "applied",
        "rules": rules.len(),
        "ingress_rules": ingress_count,
        "egress_rules": egress_count,
        "nat_entries": nat_count,
        "routes": route_count,
    })))
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

    let dhcp_pools = state
        .db
        .scan_prefix::<serde_json::Value>("dhcp_pool:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let dhcp_reservations = state
        .db
        .scan_prefix::<serde_json::Value>("dhcp_reservation:")
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    let dhcp_clients = state
        .db
        .scan_prefix::<serde_json::Value>("dhcp_client:")
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
        dhcp_pools,
        dhcp_reservations,
        dhcp_clients,
    };

    Ok(Json(backup))
}

/// Perform the actual restore operation (shared between handler and rollback).
pub async fn perform_restore(
    state: &AppState,
    backup_json: &serde_json::Value,
) -> Result<(), String> {
    let backup: BackupData =
        serde_json::from_value(backup_json.clone()).map_err(|e| format!("Invalid backup: {}", e))?;

    // Clear existing data for each prefix
    for prefix in &[
        "rule:",
        "nat:",
        "route:",
        "zone:",
        "policy:",
        "dhcp_pool:",
        "dhcp_reservation:",
        "dhcp_client:",
    ] {
        let existing = state
            .db
            .scan_prefix::<serde_json::Value>(prefix)
            .await
            .map_err(|e| e.to_string())?;
        for (key, _) in &existing {
            state.db.delete(key).await.map_err(|e| e.to_string())?;
        }
        let index_key = format!("{}__index", prefix);
        let empty: Vec<String> = Vec::new();
        state
            .db
            .put(&index_key, &empty)
            .await
            .map_err(|e| e.to_string())?;
    }

    let restore_items: &[(&str, &Vec<serde_json::Value>)] = &[
        ("rule:", &backup.rules),
        ("nat:", &backup.nat_entries),
        ("route:", &backup.routes),
        ("zone:", &backup.zones),
        ("policy:", &backup.policies),
        ("dhcp_pool:", &backup.dhcp_pools),
        ("dhcp_reservation:", &backup.dhcp_reservations),
        ("dhcp_client:", &backup.dhcp_clients),
    ];

    for (prefix, items) in restore_items {
        for item in *items {
            let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let key = format!("{}{}", prefix, id);
            state
                .db
                .put(&key, item)
                .await
                .map_err(|e| e.to_string())?;
            state
                .db
                .add_to_index(prefix, &key)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Notify DHCP server to reload after restore
    let _ = state.dhcp_pool_notify.send(());

    Ok(())
}

/// Take a snapshot of the current DB state as a BackupData JSON value.
async fn snapshot_current(state: &AppState) -> Result<serde_json::Value, String> {
    let scan = |prefix: &'static str| async move {
        state
            .db
            .scan_prefix::<serde_json::Value>(prefix)
            .await
            .map_err(|e| e.to_string())
            .map(|v| v.into_iter().map(|(_, val)| val).collect::<Vec<_>>())
    };

    let backup = BackupData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        rules: scan("rule:").await?,
        nat_entries: scan("nat:").await?,
        routes: scan("route:").await?,
        zones: scan("zone:").await?,
        policies: scan("policy:").await?,
        dhcp_pools: scan("dhcp_pool:").await?,
        dhcp_reservations: scan("dhcp_reservation:").await?,
        dhcp_clients: scan("dhcp_client:").await?,
    };

    serde_json::to_value(&backup).map_err(|e| e.to_string())
}

async fn restore_data(
    State(state): State<Arc<AppState>>,
    Json(backup): Json<BackupData>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    require_no_pending(&state).await?;

    // Snapshot current state before restore (for undo)
    let old_snapshot = snapshot_current(&state).await.map_err(internal_error)?;

    // Convert incoming backup to JSON value for perform_restore
    let backup_json = serde_json::to_value(&backup).map_err(internal_error)?;

    // Perform the restore
    perform_restore(&state, &backup_json)
        .await
        .map_err(internal_error)?;

    // Record pending change for revert
    changeset::record_full_restore(&state,
        old_snapshot,
        "Restore configuration from backup".to_string(),
    )
    .await;

    broadcast(&state, WsEvent::ConfigRestored);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "restored",
            "rules": backup.rules.len(),
            "nat_entries": backup.nat_entries.len(),
            "routes": backup.routes.len(),
            "zones": backup.zones.len(),
            "policies": backup.policies.len(),
            "dhcp_pools": backup.dhcp_pools.len(),
            "dhcp_reservations": backup.dhcp_reservations.len(),
            "dhcp_clients": backup.dhcp_clients.len(),
        })),
    ))
}

// === SNI Filtering ===

/// FNV-1a hash matching the eBPF kernel-side implementation.
/// Used to compute the hash key for the SNI_POLICY eBPF map.
fn fnv1a_hash(domain: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in domain.to_lowercase().bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

async fn list_sni_rules(State(state): State<Arc<AppState>>) -> AppResult<Vec<SniRule>> {
    let results = state
        .db
        .scan_prefix::<SniRule>("sni_rule:")
        .await
        .map_err(internal_error)?;
    Ok(Json(results.into_iter().map(|(_, r)| r).collect()))
}

async fn create_sni_rule(
    State(state): State<Arc<AppState>>,
    Json(mut rule): Json<SniRule>,
) -> Result<(StatusCode, Json<SniRule>), (StatusCode, String)> {
    require_no_pending(&state).await?;
    let existing = state
        .db
        .scan_prefix::<SniRule>("sni_rule:")
        .await
        .map_err(internal_error)?;
    let next_id = existing.iter().map(|(_, r)| r.id).max().unwrap_or(0) + 1;
    rule.id = next_id;
    rule.hit_count = 0;
    let key = format!("sni_rule:{}", rule.id);
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    state
        .db
        .add_to_index("sni_rule:", &key)
        .await
        .map_err(internal_error)?;
    changeset::record_create(
        &state,
        "sni_rule:",
        &key,
        format!("Created SNI rule '{}' for {}", rule.id, rule.domain),
    )
    .await;
    broadcast(&state, WsEvent::SniRuleCreated(to_json_value(&rule)));
    sync_sni_to_ebpf(&state).await;
    Ok((StatusCode::CREATED, Json(rule)))
}

async fn update_sni_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(mut rule): Json<SniRule>,
) -> AppResult<SniRule> {
    require_no_pending(&state).await?;
    rule.id = id;
    let key = format!("sni_rule:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    if old.is_none() {
        return Err((StatusCode::NOT_FOUND, "SNI rule not found".to_string()));
    }
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(
            &state,
            &key,
            old_val,
            format!("Updated SNI rule #{}", id),
        )
        .await;
    }
    state
        .db
        .add_to_index("sni_rule:", &key)
        .await
        .map_err(internal_error)?;
    broadcast(&state, WsEvent::SniRuleUpdated(to_json_value(&rule)));
    sync_sni_to_ebpf(&state).await;
    Ok(Json(rule))
}

async fn delete_sni_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_no_pending(&state).await?;
    let key = format!("sni_rule:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    state.db.delete(&key).await.map_err(internal_error)?;
    state
        .db
        .remove_from_index("sni_rule:", &key)
        .await
        .map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_delete(
            &state,
            "sni_rule:",
            &key,
            old_val,
            format!("Deleted SNI rule #{}", id),
        )
        .await;
    }
    broadcast(&state, WsEvent::SniRuleDeleted { id });
    sync_sni_to_ebpf(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_sni_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> AppResult<SniRule> {
    require_no_pending(&state).await?;
    let key = format!("sni_rule:{}", id);
    let old = state.db.get_raw(&key).await.map_err(internal_error)?;
    let mut rule: SniRule = state
        .db
        .get(&key)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "SNI rule not found".to_string()))?;
    rule.enabled = !rule.enabled;
    state.db.put(&key, &rule).await.map_err(internal_error)?;
    if let Some(old_val) = old {
        changeset::record_update(
            &state,
            &key,
            old_val,
            format!("Toggled SNI rule #{}", id),
        )
        .await;
    }
    broadcast(&state, WsEvent::SniRuleUpdated(to_json_value(&rule)));
    sync_sni_to_ebpf(&state).await;
    Ok(Json(rule))
}

async fn sni_stats(State(state): State<Arc<AppState>>) -> AppResult<SniStats> {
    let rules = state
        .db
        .scan_prefix::<SniRule>("sni_rule:")
        .await
        .map_err(internal_error)?;

    let mut total_blocked: u64 = 0;
    let mut total_allowed: u64 = 0;
    let mut total_logged: u64 = 0;

    for (_, rule) in &rules {
        match rule.action {
            nylon_wall_common::tls::SniAction::Block => total_blocked += rule.hit_count,
            nylon_wall_common::tls::SniAction::Allow => total_allowed += rule.hit_count,
            nylon_wall_common::tls::SniAction::Log => total_logged += rule.hit_count,
        }
    }

    // Check global SNI enabled state from DB
    let enabled = state
        .db
        .get::<bool>("sni_filtering_enabled")
        .await
        .map_err(internal_error)?
        .unwrap_or(false);

    Ok(Json(SniStats {
        total_inspected: total_blocked + total_allowed + total_logged,
        total_blocked,
        total_allowed,
        total_logged,
        enabled,
    }))
}

#[derive(Deserialize)]
struct SniToggleRequest {
    enabled: Option<bool>,
}

async fn toggle_sni_filtering(
    State(state): State<Arc<AppState>>,
    body: Option<Json<SniToggleRequest>>,
) -> AppResult<serde_json::Value> {
    let current = state
        .db
        .get::<bool>("sni_filtering_enabled")
        .await
        .map_err(internal_error)?
        .unwrap_or(false);

    let new_state = match body {
        Some(Json(req)) => req.enabled.unwrap_or(!current),
        None => !current,
    };

    state
        .db
        .put("sni_filtering_enabled", &new_state)
        .await
        .map_err(internal_error)?;

    // Update eBPF SNI_ENABLED map
    #[cfg(target_os = "linux")]
    {
        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            let map = bpf.map_mut("SNI_ENABLED");
            if let Some(map) = map {
                if let Ok(mut array) = aya::maps::Array::<_, u32>::try_from(map) {
                    let val: u32 = if new_state { 1 } else { 0 };
                    let _ = array.set(0, val, 0);
                }
            }
        }
    }

    tracing::info!("SNI filtering {}", if new_state { "enabled" } else { "disabled" });

    Ok(Json(serde_json::json!({
        "enabled": new_state
    })))
}

async fn debug_sni_maps(
    State(state): State<Arc<AppState>>,
) -> AppResult<serde_json::Value> {
    let mut result = serde_json::json!({
        "sni_enabled": null,
        "policy_entries": [],
        "expected_hashes": [],
    });

    // Show expected hashes from rules
    let rules = state.db.scan_prefix::<SniRule>("sni_rule:").await.map_err(internal_error)?;
    let mut expected: Vec<serde_json::Value> = Vec::new();
    for (_, rule) in &rules {
        let domain = if rule.domain.starts_with("*.") { &rule.domain[2..] } else { &rule.domain };
        let hash = fnv1a_hash(domain);
        expected.push(serde_json::json!({
            "domain": domain,
            "hash": hash,
            "hash_hex": format!("0x{:08x}", hash),
            "action": format!("{:?}", rule.action),
            "enabled": rule.enabled,
        }));
    }
    result["expected_hashes"] = serde_json::json!(expected);

    #[cfg(target_os = "linux")]
    {
        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            // Read SNI_ENABLED
            if let Some(map) = bpf.map_mut("SNI_ENABLED") {
                if let Ok(array) = aya::maps::Array::<_, u32>::try_from(map) {
                    if let Ok(val) = array.get(&0, 0) {
                        result["sni_enabled"] = serde_json::json!(val);
                    }
                }
            }

            // Read SNI_POLICY entries
            if let Some(map) = bpf.map_mut("SNI_POLICY") {
                if let Ok(hashmap) = aya::maps::HashMap::<_, u32, u8>::try_from(map) {
                    let mut entries: Vec<serde_json::Value> = Vec::new();
                    for item in hashmap.iter() {
                        if let Ok((key, value)) = item {
                            entries.push(serde_json::json!({
                                "hash": key,
                                "hash_hex": format!("0x{:08x}", key),
                                "action": value,
                            }));
                        }
                    }
                    result["policy_entries"] = serde_json::json!(entries);
                }
            }

        }
    }

    Ok(Json(result))
}

// === SNI eBPF Sync ===

/// Sync all enabled SNI rules to the eBPF SNI_POLICY hash map.
/// Each domain (and its wildcard variant without leading "*.")  is hashed
/// with FNV-1a and inserted with the action byte (0=allow, 1=block, 2=log).
pub async fn sync_sni_to_ebpf(state: &AppState) {
    let rules = match state.db.scan_prefix::<SniRule>("sni_rule:").await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to read SNI rules for eBPF sync: {}", e);
            return;
        }
    };

    // Build hash→action map from enabled rules
    let mut policy_entries: Vec<(u32, u8)> = Vec::new();
    for (_, rule) in &rules {
        if !rule.enabled {
            continue;
        }
        let action: u8 = match rule.action {
            nylon_wall_common::tls::SniAction::Allow => 0,
            nylon_wall_common::tls::SniAction::Block => 1,
            nylon_wall_common::tls::SniAction::Log => 2,
        };

        // Strip leading "*." for wildcard rules — eBPF checks parent domains
        let domain = if rule.domain.starts_with("*.") {
            &rule.domain[2..]
        } else {
            &rule.domain
        };

        let hash = fnv1a_hash(domain);
        policy_entries.push((hash, action));
    }

    #[cfg(target_os = "linux")]
    {
        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            if let Err(e) = crate::ebpf_loader::sync_sni_to_maps(bpf, &policy_entries) {
                tracing::error!("Failed to sync SNI policies to eBPF: {}", e);
            }

            // Also sync the global enabled state
            let enabled = state
                .db
                .get::<bool>("sni_filtering_enabled")
                .await
                .unwrap_or(None)
                .unwrap_or(false);
            let map = bpf.map_mut("SNI_ENABLED");
            if let Some(map) = map {
                if let Ok(mut array) = aya::maps::Array::<_, u32>::try_from(map) {
                    let val: u32 = if enabled { 1 } else { 0 };
                    let _ = array.set(0, val, 0);
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::debug!(
            "SNI eBPF sync skipped (not Linux): {} policy entries",
            policy_entries.len()
        );
    }
}
