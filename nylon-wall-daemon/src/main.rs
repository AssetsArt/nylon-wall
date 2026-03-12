mod api;
mod db;
mod ebpf_loader;
mod rule_engine;
mod state;

use std::sync::Arc;
use tracing::info;

pub struct AppState {
    pub db: db::Database,
    pub rule_engine: tokio::sync::RwLock<rule_engine::RuleEngine>,
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

    // Load existing rules from DB
    // TODO: rule_engine.load_from_db(&db).await?;

    let state = Arc::new(AppState {
        db,
        rule_engine: tokio::sync::RwLock::new(rule_engine),
    });

    // Start eBPF loader on Linux
    #[cfg(target_os = "linux")]
    {
        info!("Loading eBPF programs...");
        ebpf_loader::load_and_attach().await?;
        info!("eBPF programs loaded");
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
