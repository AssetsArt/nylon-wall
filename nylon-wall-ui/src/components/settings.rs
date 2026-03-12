use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Settings() -> Element {
    let status = use_resource(|| async {
        api_client::get::<SystemStatus>("/system/status").await
    });
    let mut backup_msg = use_signal(|| None::<(bool, String)>);

    rsx! {
        div {
            div { class: "mb-6",
                h2 { class: "text-xl font-semibold text-white", "Settings" }
                p { class: "text-sm text-slate-400 mt-1", "System configuration and maintenance" }
            }

            if let Some((success, msg)) = backup_msg() {
                div {
                    class: if success {
                        "mb-4 px-4 py-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-sm text-emerald-400 flex items-center justify-between"
                    } else {
                        "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 flex items-center justify-between"
                    },
                    span { "{msg}" }
                    button {
                        class: "text-slate-400 hover:text-slate-300",
                        onclick: move |_| backup_msg.set(None),
                        Icon { width: 14, height: 14, icon: LdX }
                    }
                }
            }

            // System Information
            div { class: "mb-6",
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdServer, class: "text-slate-500" }
                    h3 { class: "text-sm font-semibold text-white", "System Information" }
                }
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5",
                    match &*status.read() {
                        Some(Ok(s)) => rsx! {
                            div { class: "space-y-3",
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Version" }
                                    span { class: "text-sm text-slate-300 font-mono", "v{s.version}" }
                                }
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "eBPF Status" }
                                    span {
                                        class: if s.ebpf_loaded {
                                            "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                        } else {
                                            "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20"
                                        },
                                        if s.ebpf_loaded { "Loaded" } else { "Not Loaded" }
                                    }
                                }
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Uptime" }
                                    span { class: "text-sm text-slate-300 font-mono",
                                        {format_uptime(s.uptime_seconds)}
                                    }
                                }
                                div { class: "flex items-center justify-between py-2",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Engine" }
                                    span { class: "text-sm text-slate-300", "eBPF / XDP" }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "flex items-center gap-2 text-red-400",
                                Icon { width: 14, height: 14, icon: LdTriangleAlert }
                                span { class: "text-sm", "Failed to load status: {e}" }
                            }
                        },
                        None => rsx! {
                            p { class: "text-sm text-slate-600", "Loading..." }
                        },
                    }
                }
            }

            // Backup & Restore
            div {
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdHardDrive, class: "text-slate-500" }
                    h3 { class: "text-sm font-semibold text-white", "Backup & Restore" }
                }
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5",
                    p { class: "text-sm text-slate-400 mb-4", "Export or import your firewall configuration for backup or migration." }
                    div { class: "flex items-center gap-3",
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors flex items-center gap-1.5",
                            onclick: move |_| {
                                spawn(async move {
                                    match api_client::post::<(), serde_json::Value>("/system/backup", &()).await {
                                        Ok(_) => backup_msg.set(Some((true, "Backup created successfully".to_string()))),
                                        Err(e) => backup_msg.set(Some((false, format!("Backup failed: {}", e)))),
                                    }
                                });
                            },
                            Icon { width: 13, height: 13, icon: LdDownload }
                            "Export Configuration"
                        }
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20 hover:bg-slate-500/20 transition-colors flex items-center gap-1.5",
                            Icon { width: 13, height: 13, icon: LdUpload }
                            "Import Configuration"
                        }
                    }
                }
            }
        }
    }
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, mins, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
