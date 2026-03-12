mod api;
mod db;
mod ebpf_loader;
mod events;
mod metrics;
mod rule_engine;
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

    let state = Arc::new(AppState {
        db,
        rule_engine: tokio::sync::RwLock::new(rule_engine),
        event_tx,
        started_at: Instant::now(),
        #[cfg(target_os = "linux")]
        ebpf: tokio::sync::Mutex::new(ebpf_handle),
        ebpf_loaded: std::sync::atomic::AtomicBool::new(_ebpf_loaded),
    });

    // Sync existing rules from DB to eBPF maps on startup
    #[cfg(target_os = "linux")]
    if _ebpf_loaded {
        info!("Syncing existing rules from DB to eBPF maps...");
        api::sync_rules_to_ebpf(&state).await;
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

    // Start API server
    let listen_addr = "0.0.0.0:9450";
    info!("Starting API server on {}", listen_addr);
    api::serve(state, listen_addr).await?;

    Ok(())
}
