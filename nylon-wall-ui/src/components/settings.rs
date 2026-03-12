use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[derive(Debug, Clone, serde::Deserialize)]
struct NetworkInterface {
    name: String,
    mac: String,
    ip: String,
    status: String,
    mtu: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ApplyResult {
    status: String,
    rules: u64,
    ingress_rules: u64,
    egress_rules: u64,
    nat_entries: u64,
    routes: u64,
}

#[component]
pub fn Settings() -> Element {
    let status = use_resource(|| async {
        api_client::get::<SystemStatus>("/system/status").await
    });
    let interfaces = use_resource(|| async {
        api_client::get::<Vec<NetworkInterface>>("/system/interfaces").await
    });
    let mut backup_msg = use_signal(|| None::<(bool, String)>);
    let mut apply_msg = use_signal(|| None::<(bool, String)>);
    let mut applying = use_signal(|| false);

    // Daemon settings state
    let mut listen_addr = use_signal(|| "0.0.0.0:9450".to_string());
    let mut ebpf_iface = use_signal(|| "eth0".to_string());
    let mut log_level = use_signal(|| "info".to_string());

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

            if let Some((success, msg)) = apply_msg() {
                div {
                    class: if success {
                        "mb-4 px-4 py-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-sm text-emerald-400 flex items-center justify-between"
                    } else {
                        "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 flex items-center justify-between"
                    },
                    span { "{msg}" }
                    button {
                        class: "text-slate-400 hover:text-slate-300",
                        onclick: move |_| apply_msg.set(None),
                        Icon { width: 14, height: 14, icon: LdX }
                    }
                }
            }

            // Apply Configuration
            div { class: "mb-6",
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdZap, class: "text-amber-500" }
                    h3 { class: "text-sm font-semibold text-white", "Apply Configuration" }
                }
                div { class: "rounded-xl border border-amber-500/20 bg-slate-900/50 p-5",
                    p { class: "text-sm text-slate-400 mb-4", "Push current firewall rules, NAT entries, and routes to the eBPF datapath." }
                    button {
                        class: "px-4 py-2 rounded-lg text-sm font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20 hover:bg-amber-500/20 transition-colors flex items-center gap-2 disabled:opacity-50",
                        disabled: applying(),
                        onclick: move |_| {
                            applying.set(true);
                            spawn(async move {
                                match api_client::post::<(), ApplyResult>("/system/apply", &()).await {
                                    Ok(result) => {
                                        apply_msg.set(Some((true, format!(
                                            "Configuration applied: {} rules ({} ingress, {} egress), {} NAT, {} routes",
                                            result.rules, result.ingress_rules, result.egress_rules,
                                            result.nat_entries, result.routes
                                        ))));
                                    }
                                    Err(e) => apply_msg.set(Some((false, format!("Apply failed: {}", e)))),
                                }
                                applying.set(false);
                            });
                        },
                        Icon { width: 14, height: 14, icon: LdPlay }
                        if applying() { "Applying..." } else { "Apply Now" }
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

            // Network Interfaces
            div { class: "mb-6",
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdNetwork, class: "text-slate-500" }
                    h3 { class: "text-sm font-semibold text-white", "Network Interfaces" }
                }
                div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                    table { class: "w-full text-left",
                        thead { class: "bg-slate-900/80",
                            tr {
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Name" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "IP Address" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "MAC" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "MTU" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Zone" }
                            }
                        }
                        tbody {
                            match &*interfaces.read() {
                                Some(Ok(list)) if !list.is_empty() => rsx! {
                                    for iface in list.iter() {
                                        tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                            key: "{iface.name}",
                                            td { class: "px-5 py-3 text-sm text-slate-300 font-mono font-medium", "{iface.name}" }
                                            td { class: "px-5 py-3 text-sm text-slate-400 font-mono", "{iface.ip}" }
                                            td { class: "px-5 py-3 text-sm text-slate-500 font-mono", "{iface.mac}" }
                                            td { class: "px-5 py-3 text-sm text-slate-500 font-mono", "{iface.mtu}" }
                                            td { class: "px-5 py-3 text-sm",
                                                span {
                                                    class: if iface.status == "up" {
                                                        "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                                    } else {
                                                        "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20"
                                                    },
                                                    "{iface.status}"
                                                }
                                            }
                                            td { class: "px-5 py-3 text-sm text-slate-500",
                                                {
                                                    let zone = match iface.name.as_str() {
                                                        "eth0" => "WAN",
                                                        "eth1" => "LAN",
                                                        "lo" => "Local",
                                                        _ => "Unassigned",
                                                    };
                                                    rsx! {
                                                        span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20",
                                                            "{zone}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(Ok(_)) => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "No interfaces found" } }
                                },
                                Some(Err(e)) => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "6", "Failed to load: {e}" } }
                                },
                                None => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "Loading..." } }
                                },
                            }
                        }
                    }
                }
            }

            // Daemon Settings
            div { class: "mb-6",
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdSettings, class: "text-slate-500" }
                    h3 { class: "text-sm font-semibold text-white", "Daemon Settings" }
                }
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5",
                    div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-4",
                        div {
                            label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Listen Address" }
                            input {
                                class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors font-mono",
                                r#type: "text", value: "{listen_addr}",
                                oninput: move |e| listen_addr.set(e.value()),
                            }
                        }
                        div {
                            label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "eBPF Interface" }
                            input {
                                class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors font-mono",
                                r#type: "text", value: "{ebpf_iface}",
                                oninput: move |e| ebpf_iface.set(e.value()),
                            }
                        }
                        div {
                            label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Log Level" }
                            select {
                                class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                                value: "{log_level}",
                                onchange: move |e| log_level.set(e.value()),
                                option { value: "trace", "Trace" }
                                option { value: "debug", "Debug" }
                                option { value: "info", "Info" }
                                option { value: "warn", "Warn" }
                                option { value: "error", "Error" }
                            }
                        }
                    }
                    p { class: "text-xs text-slate-600 mt-2", "Changes to daemon settings require a daemon restart to take effect." }
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
                                        Ok(data) => {
                                            let json_str = serde_json::to_string_pretty(&data).unwrap_or_default();
                                            tracing::info!("Backup data: {} bytes", json_str.len());
                                            backup_msg.set(Some((true, format!("Backup exported ({} bytes)", json_str.len()))));
                                        }
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
