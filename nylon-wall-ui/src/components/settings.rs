use dioxus::prelude::*;
use dioxus::document;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;
use super::ConfirmModal;

#[derive(Debug, Clone, serde::Deserialize)]
struct NetworkInterface {
    name: String,
    mac: String,
    ip: String,
    status: String,
    mtu: u32,
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
    let mut importing = use_signal(|| false);
    let mut confirm_import = use_signal(|| None::<String>);

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

            if confirm_import().is_some() {
                ConfirmModal {
                    title: "Import Configuration".to_string(),
                    message: "This will replace all current rules, NAT entries, routes, zones, and policies with the imported configuration. This action cannot be undone.".to_string(),
                    confirm_label: "Import".to_string(),
                    danger: false,
                    on_confirm: move |_| {
                        if let Some(content) = confirm_import() {
                            confirm_import.set(None);
                            spawn(async move {
                                match serde_json::from_str::<serde_json::Value>(&content) {
                                    Ok(backup_data) => {
                                        match api_client::post::<serde_json::Value, serde_json::Value>("/system/restore", &backup_data).await {
                                            Ok(resp) => {
                                                let status = resp.get("status").and_then(|s| s.as_str()).unwrap_or("done");
                                                backup_msg.set(Some((true, format!("Configuration restored ({})", status))));
                                            }
                                            Err(e) => backup_msg.set(Some((false, format!("Restore failed: {}", e)))),
                                        }
                                    }
                                    Err(e) => backup_msg.set(Some((false, format!("Invalid backup file: {}", e)))),
                                }
                            });
                        }
                    },
                    on_cancel: move |_| { confirm_import.set(None); },
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

            // Network Interfaces (only those with a non-empty status)
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
                                Some(Ok(list)) => {
                                    let active: Vec<_> = list.iter().filter(|i| !i.status.is_empty() && i.status != "unknown").collect();
                                    if active.is_empty() {
                                        rsx! {
                                            tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "No active interfaces found" } }
                                        }
                                    } else {
                                        rsx! {
                                            for iface in active.iter() {
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
                                        }
                                    }
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
                                            let bytes_len = json_str.len();
                                            // Use JS to trigger a file download via Blob
                                            let js_code = format!(
                                                r#"
                                                (function() {{
                                                    var json = {};
                                                    var blob = new Blob([JSON.stringify(json, null, 2)], {{ type: 'application/json' }});
                                                    var url = URL.createObjectURL(blob);
                                                    var a = document.createElement('a');
                                                    a.href = url;
                                                    a.download = 'nylon-wall-backup.json';
                                                    document.body.appendChild(a);
                                                    a.click();
                                                    document.body.removeChild(a);
                                                    URL.revokeObjectURL(url);
                                                }})();
                                                "#,
                                                json_str
                                            );
                                            document::eval(&js_code);
                                            backup_msg.set(Some((true, format!("Backup exported ({} bytes)", bytes_len))));
                                        }
                                        Err(e) => backup_msg.set(Some((false, format!("Backup failed: {}", e)))),
                                    }
                                });
                            },
                            Icon { width: 13, height: 13, icon: LdDownload }
                            span { "Export Configuration" }
                        }
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20 hover:bg-slate-500/20 transition-colors flex items-center gap-1.5 disabled:opacity-50",
                            disabled: importing(),
                            onclick: move |_| {
                                importing.set(true);
                                spawn(async move {
                                    // Create a file input, trigger click, read file content
                                    // Uses window focus event to detect cancel (file dialog closing without selection)
                                    let js_code = r#"
                                        var input = document.createElement('input');
                                        input.type = 'file';
                                        input.accept = '.json';
                                        var handled = false;
                                        input.onchange = function(e) {
                                            handled = true;
                                            var file = e.target.files[0];
                                            if (!file) { dioxus.send(''); return; }
                                            var reader = new FileReader();
                                            reader.onload = function(ev) { dioxus.send(ev.target.result); };
                                            reader.onerror = function() { dioxus.send(''); };
                                            reader.readAsText(file);
                                        };
                                        window.addEventListener('focus', function onFocus() {
                                            window.removeEventListener('focus', onFocus);
                                            setTimeout(function() {
                                                if (!handled) { dioxus.send(''); }
                                            }, 500);
                                        });
                                        input.click();
                                    "#;
                                    let mut eval = document::eval(js_code);
                                    match eval.recv::<String>().await {
                                        Ok(file_content) => {
                                            if file_content.is_empty() {
                                                importing.set(false);
                                                return;
                                            }
                                            // Validate JSON before showing confirm
                                            match serde_json::from_str::<serde_json::Value>(&file_content) {
                                                Ok(_) => {
                                                    // Store content and show confirm modal
                                                    confirm_import.set(Some(file_content));
                                                }
                                                Err(e) => {
                                                    backup_msg.set(Some((false, format!("Invalid backup file: {}", e))));
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            backup_msg.set(Some((false, "File read cancelled or failed".to_string())));
                                        }
                                    }
                                    importing.set(false);
                                });
                            },
                            Icon { width: 13, height: 13, icon: LdUpload }
                            if importing() { span { "Importing..." } } else { span { "Import Configuration" } }
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
