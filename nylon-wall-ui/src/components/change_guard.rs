use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;
use serde::Deserialize;

use crate::api_client;

/// Fallback timeout if the daemon hasn't responded yet.
const DEFAULT_TOTAL_SECS: u32 = 6;

#[derive(Deserialize)]
struct PendingStatus {
    pending: bool,
    #[allow(dead_code)]
    description: String,
    remaining_secs: u64,
    #[serde(default = "default_total")]
    total_secs: u64,
}

fn default_total() -> u64 {
    DEFAULT_TOTAL_SECS as u64
}

/// State for pending change tracking.
#[derive(Clone, Copy, PartialEq)]
pub struct ChangeGuardState {
    pub active: bool,
    pub remaining: u32,
    /// Total countdown duration (from daemon config).
    pub total: u32,
}

impl Default for ChangeGuardState {
    fn default() -> Self {
        Self {
            active: false,
            remaining: 0,
            total: DEFAULT_TOTAL_SECS,
        }
    }
}

/// Initialize the change guard context. Call once in Layout.
pub fn use_change_guard_provider() -> Signal<ChangeGuardState> {
    let sig = use_signal(ChangeGuardState::default);
    use_context_provider(|| sig);
    sig
}

/// Get the change guard signal from context.
pub fn use_change_guard() -> Signal<ChangeGuardState> {
    use_context::<Signal<ChangeGuardState>>()
}

/// Notify that a change was made. Shows the countdown modal.
pub fn notify_change(guard: &mut Signal<ChangeGuardState>) {
    let current_total = guard().total;
    guard.set(ChangeGuardState {
        active: true,
        remaining: current_total,
        total: current_total,
    });
}

/// Centered confirmation modal with countdown timer.
/// Local 1-second ticker for smooth countdown, API poll every 3s to sync state.
/// Two actions: "Confirm Changes" and "Revert Now".
#[component]
pub fn ChangeTimerModal() -> Element {
    let mut guard = use_change_guard();
    let mut confirming = use_signal(|| false);
    let mut reverting = use_signal(|| false);
    let mut reverted = use_signal(|| false);
    let mut was_active = use_signal(|| false);
    let mut first_run = use_signal(|| true);

    // Local 1-second countdown ticker
    use_future(move || async move {
        loop {
            if first_run() {
                if let Ok(status) = api_client::get::<PendingStatus>("/changes/pending").await {
                    if status.pending {
                        guard.set(ChangeGuardState {
                            active: true,
                            remaining: status.remaining_secs as u32,
                            total: status.total_secs as u32,
                        });
                    }
                }
                first_run.set(false);
            }
            gloo_timers::future::TimeoutFuture::new(1_000).await;

            let ctx = guard();
            if !ctx.active || confirming() || reverting() || reverted() {
                continue;
            }

            if ctx.remaining > 0 {
                guard.set(ChangeGuardState {
                    remaining: ctx.remaining - 1,
                    ..ctx
                });
            } else {
                // Local countdown hit 0 — check daemon immediately
                gloo_timers::future::TimeoutFuture::new(200).await;
                if let Ok(status) = api_client::get::<PendingStatus>("/changes/pending").await {
                    if !status.pending {
                        was_active.set(false);
                        guard.set(ChangeGuardState {
                            active: false,
                            remaining: 0,
                            total: status.total_secs as u32,
                        });
                        reverted.set(true);
                    }
                }
            }
        }
    });

    // API poll every 3 seconds — sync with daemon and detect auto-revert
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(3_000).await;

            if confirming() || reverting() || reverted() {
                continue;
            }

            if let Ok(status) = api_client::get::<PendingStatus>("/changes/pending").await {
                if status.pending {
                    was_active.set(true);
                    // Sync remaining from daemon (corrects any local drift)
                    guard.set(ChangeGuardState {
                        active: true,
                        remaining: status.remaining_secs as u32,
                        total: status.total_secs as u32,
                    });
                } else if was_active() {
                    // Was active but daemon says not pending = auto-reverted
                    was_active.set(false);
                    guard.set(ChangeGuardState {
                        active: false,
                        remaining: 0,
                        total: status.total_secs as u32,
                    });
                    reverted.set(true);
                } else {
                    // Not active — just keep total_secs in sync
                    let ctx = guard();
                    if !ctx.active {
                        guard.set(ChangeGuardState {
                            total: status.total_secs as u32,
                            ..ctx
                        });
                    }
                }
            }
        }
    });

    let ctx = guard();

    // Show "reverted" result modal
    if reverted() {
        return rsx! {
            div { class: "fixed inset-0 z-50 flex items-center justify-center",
                div { class: "absolute inset-0 bg-black/60 backdrop-blur-sm" }
                div { class: "relative z-10 w-full max-w-md mx-4 rounded-2xl border border-red-500/30 bg-slate-900 shadow-2xl",
                    div { class: "p-6",
                        div { class: "flex items-start gap-4 mb-4",
                            div { class: "flex-shrink-0 w-10 h-10 rounded-xl bg-red-500/10 border border-red-500/20 flex items-center justify-center",
                                Icon { width: 20, height: 20, icon: LdRotateCcw, class: "text-red-400" }
                            }
                            div {
                                h3 { class: "text-base font-semibold text-white mb-1", "Changes Reverted" }
                                p { class: "text-sm text-slate-400 leading-relaxed",
                                    "All pending changes have been reverted to their previous state."
                                }
                            }
                        }
                        div { class: "flex items-center justify-end mt-6",
                            button {
                                class: "px-4 py-2 rounded-lg text-sm font-medium bg-slate-800/50 text-slate-300 border border-slate-700/40 hover:bg-slate-700/50 transition-colors",
                                onclick: move |_| {
                                    reverted.set(false);
                                    let _ = document::eval("window.location.reload()");
                                },
                                "OK"
                            }
                        }
                    }
                }
            }
        };
    }

    if !ctx.active {
        return rsx! {};
    }

    let total = if ctx.total > 0 {
        ctx.total
    } else {
        DEFAULT_TOTAL_SECS
    };
    let progress_pct = (ctx.remaining as f64 / total as f64) * 100.0;

    let on_confirm = move |_| {
        confirming.set(true);
        spawn(async move {
            let _ = api_client::post::<(), serde_json::Value>("/changes/confirm", &()).await;
            was_active.set(false);
            guard.set(ChangeGuardState::default());
            confirming.set(false);
        });
    };

    let on_revert = move |_| {
        reverting.set(true);
        spawn(async move {
            let _ = api_client::post::<(), serde_json::Value>("/changes/revert", &()).await;
            was_active.set(false);
            guard.set(ChangeGuardState::default());
            reverting.set(false);
            reverted.set(true);
        });
    };

    let busy = confirming() || reverting();

    rsx! {
        // Backdrop
        div { class: "fixed inset-0 z-50 flex items-center justify-center",
            div { class: "absolute inset-0 bg-black/60 backdrop-blur-sm" }
            // Modal card
            div { class: "relative z-10 w-full max-w-md mx-4 rounded-2xl border border-slate-700/60 bg-slate-900 shadow-2xl overflow-hidden",
                // Progress bar
                div { class: "h-1.5 bg-slate-800",
                    div {
                        class: "h-full bg-gradient-to-r from-amber-500 to-amber-400 transition-all duration-1000 ease-linear",
                        style: "width: {progress_pct}%",
                    }
                }
                div { class: "p-6",
                    // Icon + title + countdown
                    div { class: "flex items-start gap-4 mb-4",
                        div { class: "flex-shrink-0 w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center",
                            Icon { width: 20, height: 20, icon: LdTimer, class: "text-amber-400" }
                        }
                        div { class: "flex-1",
                            h3 { class: "text-base font-semibold text-white mb-1", "Confirm Changes" }
                            p { class: "text-sm text-slate-400 leading-relaxed",
                                "Changes will be automatically reverted in "
                                span { class: "text-amber-400 font-bold tabular-nums", "{ctx.remaining}" }
                                " seconds if not confirmed."
                            }
                        }
                    }
                    // Buttons
                    div { class: "flex items-center justify-end gap-3 mt-6",
                        button {
                            class: "px-4 py-2 rounded-lg text-sm font-medium bg-red-500/20 text-red-400 border border-red-500/30 hover:bg-red-500/30 transition-colors disabled:opacity-50",
                            disabled: busy,
                            onclick: on_revert,
                            if reverting() { "Reverting..." } else { "Revert Now" }
                        }
                        button {
                            class: "px-4 py-2 rounded-lg text-sm font-medium bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 hover:bg-emerald-500/30 transition-colors disabled:opacity-50",
                            disabled: busy,
                            onclick: on_confirm,
                            if confirming() { "Confirming..." } else { "Confirm Changes" }
                        }
                    }
                }
            }
        }
    }
}
