use super::{ConfirmModal, use_change_guard, use_refresh_trigger, notify_change};
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::document;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

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
    let mut status = use_resource(|| async { api_client::get::<SystemStatus>("/system/status").await });
    let mut interfaces = use_resource(|| async {
        api_client::get::<Vec<NetworkInterface>>("/system/interfaces").await
    });
    let mut guard = use_change_guard();
    let mut backup_msg = use_signal(|| None::<(bool, String)>);
    let mut importing = use_signal(|| false);
    let mut confirm_import = use_signal(|| None::<String>);

    let refresh = use_refresh_trigger();
    let mut prev_refresh = use_signal(|| refresh());
    use_effect(move || {
        let r = refresh();
        if r != prev_refresh() {
            prev_refresh.set(r);
            status.restart();
            interfaces.restart();
        }
    });

    rsx! {
        div {
            PageHeader {
                title: "Settings".to_string(),
                subtitle: "System configuration and maintenance".to_string(),
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
                                                notify_change(&mut guard);
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
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdServer, class: "text-slate-500" } },
                    title: "System Information".to_string(),
                }
                FormCard { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-0",
                    match &*status.read() {
                        Some(Ok(s)) => rsx! {
                            div { class: "space-y-3",
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Version" }
                                    span { class: "text-sm text-slate-300 font-mono", "v{s.version}" }
                                }
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "eBPF Status" }
                                    Badge {
                                        color: if s.ebpf_loaded { Color::Emerald } else { Color::Red },
                                        label: if s.ebpf_loaded { "Loaded".to_string() } else { "Not Loaded".to_string() },
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

            // eBPF Programs
            match &*status.read() {
                Some(Ok(s)) if s.ebpf_loaded && !s.ebpf_programs.is_empty() => rsx! {
                    div { class: "mb-6",
                        SectionHeader {
                            icon: rsx! { Icon { width: 15, height: 15, icon: LdCpu, class: "text-slate-500" } },
                            title: "eBPF Programs".to_string(),
                        }
                        DataTable {
                            thead { class: "bg-slate-900/80",
                                tr {
                                    th { class: TH_CLASS, "Program" }
                                    th { class: TH_CLASS, "Type" }
                                    th { class: TH_CLASS, "Role" }
                                    th { class: TH_CLASS, "Stage" }
                                    th { class: TH_CLASS, "Status" }
                                }
                            }
                            tbody {
                                for prog in s.ebpf_programs.iter() {
                                    tr { class: TR_CLASS,
                                        key: "{prog.name}",
                                        td { class: "{TD_CLASS} text-slate-300 font-mono text-xs", "{prog.name}" }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if prog.prog_type == "XDP" { Color::Cyan } else { Color::Violet },
                                                label: prog.prog_type.clone(),
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if prog.role == "entry" { Color::Blue } else { Color::Slate },
                                                label: prog.role.clone(),
                                            }
                                        }
                                        td { class: "{TD_CLASS} text-slate-500 font-mono text-xs",
                                            match prog.stage {
                                                Some(0) => rsx! { "NAT" },
                                                Some(1) => rsx! { "SNI" },
                                                Some(2) => rsx! { "Rules" },
                                                Some(n) => rsx! { "{n}" },
                                                None => rsx! { span { class: "text-slate-700", "\u{2014}" } },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: Color::Emerald,
                                                label: "Loaded".to_string(),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! {},
            }

            // Network Interfaces (only those with a non-empty status)
            div { class: "mb-6",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdNetwork, class: "text-slate-500" } },
                    title: "Network Interfaces".to_string(),
                }
                DataTable {
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: TH_CLASS, "Name" }
                            th { class: TH_CLASS, "IP Address" }
                            th { class: TH_CLASS, "MAC" }
                            th { class: TH_CLASS, "MTU" }
                            th { class: TH_CLASS, "Status" }
                            th { class: TH_CLASS, "Zone" }
                        }
                    }
                    tbody {
                        match &*interfaces.read() {
                            Some(Ok(list)) => {
                                let active: Vec<_> = list.iter().filter(|i| !i.status.is_empty() && i.status != "unknown").collect();
                                if active.is_empty() {
                                    rsx! {
                                        TableEmpty { colspan: 6, message: "No active interfaces found".to_string() }
                                    }
                                } else {
                                    rsx! {
                                        for iface in active.iter() {
                                            tr { class: TR_CLASS,
                                                key: "{iface.name}",
                                                td { class: "{TD_CLASS} text-slate-300 font-mono font-medium", "{iface.name}" }
                                                td { class: "{TD_CLASS} text-slate-400 font-mono", "{iface.ip}" }
                                                td { class: "{TD_CLASS} text-slate-500 font-mono", "{iface.mac}" }
                                                td { class: "{TD_CLASS} text-slate-500 font-mono", "{iface.mtu}" }
                                                td { class: TD_CLASS,
                                                    Badge {
                                                        color: if iface.status == "up" { Color::Emerald } else { Color::Slate },
                                                        label: iface.status.clone(),
                                                    }
                                                }
                                                td { class: TD_CLASS,
                                                    {
                                                        let zone = match iface.name.as_str() {
                                                            "eth0" => "WAN",
                                                            "eth1" => "LAN",
                                                            "lo" => "Local",
                                                            _ => "Unassigned",
                                                        };
                                                        rsx! {
                                                            Badge { color: Color::Blue, label: zone.to_string() }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Err(e)) => rsx! {
                                TableError { colspan: 6, message: format!("Failed to load: {e}") }
                            },
                            None => rsx! {
                                TableLoading { colspan: 6 }
                            },
                        }
                    }
                }
            }

            // Backup & Restore
            div {
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdHardDrive, class: "text-slate-500" } },
                    title: "Backup & Restore".to_string(),
                }
                FormCard { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-0",
                    p { class: "text-sm text-slate-400 mb-4", "Export or import your firewall configuration for backup or migration." }
                    div { class: "flex items-center gap-3",
                        Btn {
                            color: Color::Blue,
                            label: "Export Configuration".to_string(),
                            icon: rsx! { Icon { width: 13, height: 13, icon: LdDownload } },
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
                        }
                        Btn {
                            color: Color::Slate,
                            label: if importing() { "Importing...".to_string() } else { "Import Configuration".to_string() },
                            disabled: importing(),
                            icon: rsx! { Icon { width: 13, height: 13, icon: LdUpload } },
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
