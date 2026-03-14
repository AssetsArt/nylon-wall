use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tracing::{info, warn};

use crate::AppState;

/// Default revert timeout (seconds). Overridden by config.toml `[changes].revert_timeout_secs`.
const DEFAULT_REVERT_TIMEOUT_SECS: u64 = 6;

/// DB key used to persist the undo action across daemon restarts.
const PENDING_UNDO_KEY: &str = "__pending_undo";

/// Runtime-configurable revert timeout, set once at startup.
static REVERT_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_REVERT_TIMEOUT_SECS);

/// Set the revert timeout (call once at startup from config).
pub fn set_revert_timeout(secs: u64) {
    REVERT_TIMEOUT.store(secs, Ordering::Relaxed);
}

/// Get the current revert timeout in seconds.
pub fn revert_timeout_secs() -> u64 {
    REVERT_TIMEOUT.load(Ordering::Relaxed)
}

/// Describes how to undo a single change.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UndoAction {
    /// Undo a create: delete the key and remove from index.
    Create { prefix: String, key: String },
    /// Undo an update: restore the old value.
    Update {
        key: String,
        old_value: serde_json::Value,
    },
    /// Undo a delete: re-create with old value and add to index.
    Delete {
        prefix: String,
        key: String,
        old_value: serde_json::Value,
    },
    /// Undo a full restore: wipe current data and re-import old snapshot.
    FullRestore { old_snapshot: serde_json::Value },
}

/// Persisted form of a pending change (survives daemon restart).
#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedUndo {
    action: UndoAction,
    description: String,
}

/// A single pending (unconfirmed) change.
pub struct PendingChange {
    pub action: UndoAction,
    pub description: String,
    pub deadline: Instant,
}

impl PendingChange {
    pub fn new(action: UndoAction, description: String) -> Self {
        Self {
            action,
            description,
            deadline: Instant::now() + Duration::from_secs(revert_timeout_secs()),
        }
    }

    pub fn remaining_secs(&self) -> u64 {
        self.deadline
            .saturating_duration_since(Instant::now())
            .as_secs()
    }
}

/// Persist the undo action to DB so it survives crashes.
async fn persist_undo(state: &AppState, action: &UndoAction, description: &str) {
    let record = PersistedUndo {
        action: action.clone(),
        description: description.to_string(),
    };
    if let Err(e) = state.db.put(PENDING_UNDO_KEY, &record).await {
        warn!("Failed to persist undo to DB: {}", e);
    }
}

/// Remove persisted undo from DB (called on confirm or after rollback).
async fn clear_persisted_undo(state: &AppState) {
    if let Err(e) = state.db.delete(PENDING_UNDO_KEY).await {
        warn!("Failed to clear persisted undo: {}", e);
    }
}

/// Check if there's already a pending change (blocks new mutations).
pub async fn has_pending(pending: &Mutex<Option<PendingChange>>) -> bool {
    pending.lock().await.is_some()
}

/// Record a create (undo = delete). Replaces any existing pending change.
pub async fn record_create(
    state: &AppState,
    prefix: &str,
    key: &str,
    description: String,
) {
    let action = UndoAction::Create {
        prefix: prefix.to_string(),
        key: key.to_string(),
    };
    persist_undo(state, &action, &description).await;
    let mut guard = state.pending_changes.lock().await;
    *guard = Some(PendingChange::new(action, description));
}

/// Record an update (undo = restore old value).
pub async fn record_update(
    state: &AppState,
    key: &str,
    old_value: serde_json::Value,
    description: String,
) {
    let action = UndoAction::Update {
        key: key.to_string(),
        old_value,
    };
    persist_undo(state, &action, &description).await;
    let mut guard = state.pending_changes.lock().await;
    *guard = Some(PendingChange::new(action, description));
}

/// Record a delete (undo = re-create).
pub async fn record_delete(
    state: &AppState,
    prefix: &str,
    key: &str,
    old_value: serde_json::Value,
    description: String,
) {
    let action = UndoAction::Delete {
        prefix: prefix.to_string(),
        key: key.to_string(),
        old_value,
    };
    persist_undo(state, &action, &description).await;
    let mut guard = state.pending_changes.lock().await;
    *guard = Some(PendingChange::new(action, description));
}

/// Record a full restore (undo = restore old snapshot).
pub async fn record_full_restore(
    state: &AppState,
    old_snapshot: serde_json::Value,
    description: String,
) {
    let action = UndoAction::FullRestore { old_snapshot };
    persist_undo(state, &action, &description).await;
    let mut guard = state.pending_changes.lock().await;
    *guard = Some(PendingChange::new(action, description));
}

/// Confirm the pending change (discard undo).
pub async fn confirm(state: &AppState) -> bool {
    let mut guard = state.pending_changes.lock().await;
    let had = guard.is_some();
    *guard = None;
    drop(guard);
    clear_persisted_undo(state).await;
    had
}

/// Get status: (description, remaining_secs). None if nothing pending.
pub async fn status(pending: &Mutex<Option<PendingChange>>) -> Option<(String, u64)> {
    let guard = pending.lock().await;
    guard
        .as_ref()
        .map(|pc| (pc.description.clone(), pc.remaining_secs()))
}

/// Revert the single pending change. Returns true if something was reverted.
pub async fn rollback(state: &AppState) -> Result<bool, String> {
    let change = {
        let mut guard = state.pending_changes.lock().await;
        guard.take()
    };

    let change = match change {
        Some(c) => c,
        None => return Ok(false),
    };

    execute_undo(state, &change.action).await;
    clear_persisted_undo(state).await;

    info!("Rolled back change: {}", change.description);
    Ok(true)
}

/// On daemon startup, check for a persisted undo and auto-revert it.
/// This covers the case where the daemon crashed while a change was pending.
pub async fn recover_pending(state: &AppState) {
    let persisted: Option<PersistedUndo> = match state.db.get(PENDING_UNDO_KEY).await {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to check for persisted undo: {}", e);
            return;
        }
    };

    if let Some(undo) = persisted {
        warn!(
            "Found un-confirmed change from previous run: {}. Auto-reverting...",
            undo.description
        );
        execute_undo(state, &undo.action).await;
        clear_persisted_undo(state).await;
        info!("Startup recovery: reverted pending change successfully");
    }
}

/// Execute an undo action against the database.
async fn execute_undo(state: &AppState, action: &UndoAction) {
    match action {
        UndoAction::Create { prefix, key } => {
            if let Err(e) = state.db.delete(key).await {
                warn!("Rollback: failed to delete {}: {}", key, e);
            }
            if let Err(e) = state.db.remove_from_index(prefix, key).await {
                warn!("Rollback: failed to update index for {}: {}", prefix, e);
            }
        }
        UndoAction::Update { key, old_value } => {
            if let Err(e) = state.db.put_raw(key, old_value).await {
                warn!("Rollback: failed to restore {}: {}", key, e);
            }
        }
        UndoAction::Delete {
            prefix,
            key,
            old_value,
        } => {
            if let Err(e) = state.db.put_raw(key, old_value).await {
                warn!("Rollback: failed to re-create {}: {}", key, e);
            }
            if let Err(e) = state.db.add_to_index(prefix, key).await {
                warn!("Rollback: failed to update index for {}: {}", prefix, e);
            }
        }
        UndoAction::FullRestore { old_snapshot } => {
            if let Err(e) = crate::api::perform_restore(state, old_snapshot).await {
                warn!("Rollback: full restore failed: {}", e);
            }
        }
    }
}

/// Background task that checks for expired pending change and auto-reverts.
pub fn spawn_auto_revert_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let should_revert = {
                let guard = state.pending_changes.lock().await;
                guard
                    .as_ref()
                    .is_some_and(|pc| pc.deadline <= Instant::now())
            };

            if should_revert {
                match rollback(&state).await {
                    Ok(true) => {
                        warn!("Auto-reverted pending change (timeout expired)");

                        // Re-sync eBPF maps after revert
                        crate::api::sync_rules_to_ebpf(&state).await;
                        crate::api::sync_nat_to_ebpf(&state).await;
                        crate::api::sync_zones_to_ebpf(&state).await;

                        // Broadcast revert event so UI can refresh
                        let _ = state
                            .event_tx
                            .send(crate::events::WsEvent::ChangesReverted { count: 1 });

                        info!("Post-revert sync completed");
                    }
                    Err(e) => warn!("Auto-revert failed: {}", e),
                    _ => {}
                }
            }
        }
    });
}
