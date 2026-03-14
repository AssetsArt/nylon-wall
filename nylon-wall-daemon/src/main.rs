mod api;
mod db;
#[allow(dead_code)] // DHCP internals used only on Linux
mod dhcp;
mod ebpf_loader;
mod events;
mod metrics;
mod nat;
mod route;
mod rule_engine;
#[allow(dead_code)] // schedule functions used at runtime via policy engine
mod schedule;
mod state;

use std::sync::Arc;
use std::time::Instant;
use tracing::info;

pub struct AppState {
    pub db: db::Database,
    pub rule_engine: tokio::sync::RwLock<rule_engine::RuleEngine>,
    pub event_tx: tokio::sync::broadcast::Sender<events::WsEvent>,
    pub started_at: Instant,
    #[cfg(target_os = "linux")]
    pub ebpf: tokio::sync::Mutex<Option<aya::Ebpf>>,
    pub ebpf_loaded: std::sync::atomic::AtomicBool,
    pub dhcp_pool_notify: tokio::sync::watch::Sender<()>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nylon_wall_daemon=info".into()),
        )
        .init();

    info!("Starting Nylon Wall daemon...");

    // Initialize database
    let db = db::Database::open("/tmp/nylon-wall/slatedb").await?;
    info!("Database initialized");

    // Initialize rule engine
    let rule_engine = rule_engine::RuleEngine::new();

    // Create broadcast channel for WebSocket events (256 event buffer)
    let (event_tx, _) = tokio::sync::broadcast::channel::<events::WsEvent>(256);

    let mut _ebpf_loaded = false;

    #[cfg(target_os = "linux")]
    let ebpf_handle = {
        info!("Loading eBPF programs...");
        match ebpf_loader::load_and_attach().await {
            Ok(bpf) => {
                info!("eBPF programs loaded");
                _ebpf_loaded = true;
                Some(bpf)
            }
            Err(e) => {
                tracing::warn!("eBPF load failed (running in demo mode): {}", e);
                None
            }
        }
    };

    // Create DHCP pool notification channel
    let (dhcp_pool_tx, _dhcp_pool_rx) = tokio::sync::watch::channel(());

    let state = Arc::new(AppState {
        db,
        rule_engine: tokio::sync::RwLock::new(rule_engine),
        event_tx,
        started_at: Instant::now(),
        #[cfg(target_os = "linux")]
        ebpf: tokio::sync::Mutex::new(ebpf_handle),
        ebpf_loaded: std::sync::atomic::AtomicBool::new(_ebpf_loaded),
        dhcp_pool_notify: dhcp_pool_tx,
    });

    // Sync existing rules from DB to eBPF maps on startup
    #[cfg(target_os = "linux")]
    if _ebpf_loaded {
        info!("Syncing existing rules from DB to eBPF maps...");
        api::sync_rules_to_ebpf(&state).await;
        api::sync_zones_to_ebpf(&state).await;
    }

    // Sync routes to kernel on startup (Linux only)
    #[cfg(target_os = "linux")]
    {
        info!("Syncing routes to kernel...");
        route::sync_routes(&state).await;
    }

    // Start perf event reader background task on Linux
    #[cfg(target_os = "linux")]
    if _ebpf_loaded {
        let perf_state = Arc::clone(&state);
        tokio::spawn(async move {
            state::perf_event_loop(perf_state).await;
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        info!("Not running on Linux - eBPF programs will not be loaded");
    }

    // Start log TTL cleanup background task
    {
        let cleanup_state = Arc::clone(&state);
        tokio::spawn(async move {
            log_ttl_cleanup(cleanup_state).await;
        });
        info!("Log TTL cleanup task spawned");
    }

    // Start DHCP server background task
    {
        let dhcp_state = Arc::clone(&state);
        let dhcp_pool_rx = state.dhcp_pool_notify.subscribe();
        tokio::spawn(async move {
            dhcp::server::run_dhcp_server(dhcp_state, dhcp_pool_rx).await;
        });
        info!("DHCP server task spawned");
    }

    // Start DHCP client tasks for enabled configs
    {
        let configs = state
            .db
            .scan_prefix::<nylon_wall_common::dhcp::DhcpClientConfig>("dhcp_client:")
            .await
            .unwrap_or_default();
        for (_, config) in configs {
            if config.enabled {
                let client_state = Arc::clone(&state);
                tokio::spawn(async move {
                    dhcp::client::run_dhcp_client(client_state, config).await;
                });
            }
        }
        info!("DHCP client tasks spawned");
    }

    // Start API server
    let listen_addr = "0.0.0.0:9450";
    info!("Starting API server on {}", listen_addr);
    api::serve(state, listen_addr).await?;

    Ok(())
}

/// Background task to clean up old log entries based on TTL.
/// Default TTL: 7 days (604800 seconds).
async fn log_ttl_cleanup(state: Arc<AppState>) {
    use nylon_wall_common::log::PacketLog;

    let ttl_seconds: i64 = 604800; // 7 days
    let cleanup_interval = tokio::time::Duration::from_secs(3600); // Run every hour

    loop {
        tokio::time::sleep(cleanup_interval).await;

        let now = chrono::Utc::now().timestamp();
        let cutoff = now - ttl_seconds;

        let logs = match state.db.scan_prefix::<PacketLog>("log:").await {
            Ok(logs) => logs,
            Err(e) => {
                tracing::warn!("Log cleanup: failed to scan logs: {}", e);
                continue;
            }
        };

        let mut removed = 0u32;
        for (key, log) in &logs {
            if log.timestamp < cutoff {
                if let Err(e) = state.db.delete(key).await {
                    tracing::warn!("Log cleanup: failed to delete {}: {}", key, e);
                }
                if let Err(e) = state.db.remove_from_index("log:", key).await {
                    tracing::warn!("Log cleanup: failed to update index: {}", e);
                }
                removed += 1;
            }
        }

        if removed > 0 {
            info!("Log TTL cleanup: removed {} expired entries", removed);
        }
    }
}
