use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use nylon_wall_common::nat::NatEntry;
use nylon_wall_common::route::Route;
use nylon_wall_common::rule::FirewallRule;
use nylon_wall_common::zone::{NetworkPolicy, Zone};
use tower_http::cors::CorsLayer;

use crate::AppState;

type AppResult<T> = Result<Json<T>, (StatusCode, String)>;

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub async fn serve(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        // Rules
        .route("/api/v1/rules", get(list_rules).post(create_rule))
        .route(
            "/api/v1/rules/{id}",
            get(get_rule).put(update_rule).delete(delete_rule),
        )
        .route("/api/v1/rules/{id}/toggle", post(toggle_rule))
        // NAT
        .route("/api/v1/nat", get(list_nat).post(create_nat))
        .route("/api/v1/nat/{id}", put(update_nat).delete(delete_nat))
        // Routes
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
        // System
        .route("/api/v1/system/status", get(system_status))
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
