pub mod api;
pub mod auth;
pub mod changeset;
pub mod db;
#[allow(dead_code)]
pub mod dhcp;
pub mod ebpf_loader;
pub mod events;
pub mod metrics;
pub mod nat;
pub mod route;
pub mod rule_engine;
#[allow(dead_code)]
pub mod schedule;
pub mod state;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

pub struct AppState {
    pub db: db::Database,
    pub rule_engine: tokio::sync::RwLock<rule_engine::RuleEngine>,
    pub event_tx: tokio::sync::broadcast::Sender<events::WsEvent>,
    pub started_at: Instant,
    #[cfg(target_os = "linux")]
    pub ebpf: tokio::sync::Mutex<Option<aya::Ebpf>>,
    pub ebpf_loaded: std::sync::atomic::AtomicBool,
    pub dhcp_pool_notify: tokio::sync::watch::Sender<()>,
    pub pending_changes: tokio::sync::Mutex<Option<changeset::PendingChange>>,
    pub jwt_keys: auth::JwtKeys,
    pub revoked_tokens: tokio::sync::RwLock<HashSet<String>>,
    pub login_tracker: auth::LoginTracker,
}

/// Create an `AppState` for testing (demo mode, no eBPF).
pub async fn create_test_state(db_path: &str) -> Arc<AppState> {
    let db = db::Database::open(db_path).await.expect("failed to open test DB");
    let jwt_keys = auth::load_or_create_jwt_keys(&db).await;
    let (event_tx, _) = tokio::sync::broadcast::channel::<events::WsEvent>(256);
    let (dhcp_pool_tx, _) = tokio::sync::watch::channel(());
    Arc::new(AppState {
        db,
        rule_engine: tokio::sync::RwLock::new(rule_engine::RuleEngine::new()),
        event_tx,
        started_at: Instant::now(),
        #[cfg(target_os = "linux")]
        ebpf: tokio::sync::Mutex::new(None),
        ebpf_loaded: std::sync::atomic::AtomicBool::new(false),
        dhcp_pool_notify: dhcp_pool_tx,
        pending_changes: tokio::sync::Mutex::new(None),
        jwt_keys,
        revoked_tokens: tokio::sync::RwLock::new(HashSet::new()),
        login_tracker: auth::LoginTracker::new(),
    })
}
