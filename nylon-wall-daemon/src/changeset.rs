use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tracing::{info, warn};

use crate::AppState;

/// How long to wait before auto-reverting (seconds).
pub const REVERT_TIMEOUT_SECS: u64 = 15;

/// Describes how to undo a single change.
#[derive(Debug, Clone)]
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
            deadline: Instant::now() + Duration::from_secs(REVERT_TIMEOUT_SECS),
        }
    }

    pub fn remaining_secs(&self) -> u64 {
        self.deadline
            .saturating_duration_since(Instant::now())
            .as_secs()
    }
}

/// Check if there's already a pending change (blocks new mutations).
pub async fn has_pending(pending: &Mutex<Option<PendingChange>>) -> bool {
    pending.lock().await.is_some()
}

/// Record a create (undo = delete). Replaces any existing pending change.
pub async fn record_create(
    pending: &Mutex<Option<PendingChange>>,
    prefix: &str,
    key: &str,
    description: String,
) {
    let mut guard = pending.lock().await;
    *guard = Some(PendingChange::new(
        UndoAction::Create {
            prefix: prefix.to_string(),
            key: key.to_string(),
        },
        description,
    ));
}

/// Record an update (undo = restore old value).
pub async fn record_update(
    pending: &Mutex<Option<PendingChange>>,
    key: &str,
    old_value: serde_json::Value,
    description: String,
) {
    let mut guard = pending.lock().await;
    *guard = Some(PendingChange::new(
        UndoAction::Update {
            key: key.to_string(),
            old_value,
        },
        description,
    ));
}

/// Record a delete (undo = re-create).
pub async fn record_delete(
    pending: &Mutex<Option<PendingChange>>,
    prefix: &str,
    key: &str,
    old_value: serde_json::Value,
    description: String,
) {
    let mut guard = pending.lock().await;
    *guard = Some(PendingChange::new(
        UndoAction::Delete {
            prefix: prefix.to_string(),
            key: key.to_string(),
            old_value,
        },
        description,
    ));
}

/// Confirm the pending change (discard undo).
pub async fn confirm(pending: &Mutex<Option<PendingChange>>) -> bool {
    let mut guard = pending.lock().await;
    let had = guard.is_some();
    *guard = None;
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

    match change.action {
        UndoAction::Create { prefix, key } => {
            if let Err(e) = state.db.delete(&key).await {
                warn!("Rollback: failed to delete {}: {}", key, e);
            }
            if let Err(e) = state.db.remove_from_index(&prefix, &key).await {
                warn!("Rollback: failed to update index for {}: {}", prefix, e);
            }
        }
        UndoAction::Update { key, old_value } => {
            if let Err(e) = state.db.put_raw(&key, &old_value).await {
                warn!("Rollback: failed to restore {}: {}", key, e);
            }
        }
        UndoAction::Delete {
            prefix,
            key,
            old_value,
        } => {
            if let Err(e) = state.db.put_raw(&key, &old_value).await {
                warn!("Rollback: failed to re-create {}: {}", key, e);
            }
            if let Err(e) = state.db.add_to_index(&prefix, &key).await {
                warn!("Rollback: failed to update index for {}: {}", prefix, e);
            }
        }
    }

    info!("Rolled back change: {}", change.description);
    Ok(true)
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
                        crate::api::sync_zones_to_ebpf(&state).await;

                        // Broadcast revert event so UI can refresh
                        let _ = state.event_tx.send(crate::events::WsEvent::ChangesReverted {
                            count: 1,
                        });

                        info!("Post-revert sync completed");
                    }
                    Err(e) => warn!("Auto-revert failed: {}", e),
                    _ => {}
                }
            }
        }
    });
}
