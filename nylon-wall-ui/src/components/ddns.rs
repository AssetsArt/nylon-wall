use super::ui::*;
use super::{ConfirmModal, notify_change, use_change_guard, use_refresh_trigger};
use crate::api_client;
use crate::models::{DdnsEntry, DdnsProvider, DdnsStatus};
use crate::ws_client;
use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;

#[component]
pub fn Ddns() -> Element {
    let refresh = use_refresh_trigger();
    let ws = ws_client::use_ws_events();
    let mut guard = use_change_guard();

    let mut entries = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.ddns();
        api_client::get::<Vec<DdnsEntry>>("/ddns").await
    });

    let mut statuses = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.ddns();
        api_client::get::<Vec<DdnsStatus>>("/ddns/status").await
    });

    let mut editing = use_signal(|| None::<(bool, DdnsEntry)>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut updating_id = use_signal(|| None::<u32>);
    let mut error_msg = use_signal(|| None::<String>);

    let status_map: std::collections::HashMap<u32, DdnsStatus> = statuses
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|s| s.iter().map(|st| (st.id, st.clone())).collect())
        .unwrap_or_default();

    rsx! {
        div {
            PageHeader {
                title: "Dynamic DNS".to_string(),
                subtitle: "Automatically update DNS records when your WAN IP changes.".to_string(),
                Btn {
                    color: Color::Blue,
                    label: if editing().is_some() {
                        "Cancel".to_string()
                    } else {
                        "+ Add Entry".to_string()
                    },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((
                                false,
                                DdnsEntry {
                                    id: 0,
                                    provider: DdnsProvider::Cloudflare,
                                    hostname: String::new(),
                                    username: String::new(),
                                    token: String::new(),
                                    custom_url: String::new(),
                                    interval_secs: 300,
                                    enabled: true,
                                },
                            )));
                        }
                    },
                }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            // Stats cards
            {
                let entries_data = entries.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
                let status_data = statuses.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
                rsx! {
                    div { class: "grid grid-cols-3 gap-4 mb-6",
                        StatCard {
                            label: "Entries".to_string(),
                            value: entries_data.as_ref().map(|e| e.len().to_string()).unwrap_or("-".to_string()),
                            color: Color::Blue,
                            icon: rsx! { Icon { width: 16, height: 16, icon: LdGlobe } },
                        }
                        StatCard {
                            label: "Active".to_string(),
                            value: entries_data.as_ref().map(|e| e.iter().filter(|x| x.enabled).count().to_string()).unwrap_or("-".to_string()),
                            color: Color::Emerald,
                            icon: rsx! { Icon { width: 16, height: 16, icon: LdActivity } },
                        }
                        StatCard {
                            label: "Total Updates".to_string(),
                            value: status_data.as_ref().map(|s| s.iter().map(|x| x.update_count).sum::<u64>().to_string()).unwrap_or("-".to_string()),
                            color: Color::Violet,
                            icon: rsx! { Icon { width: 16, height: 16, icon: LdRefreshCw } },
                        }
                    }
                }
            }

            // Edit / Create form
            if let Some((is_edit, entry)) = editing() {
                DdnsForm {
                    is_edit: is_edit,
                    entry: entry,
                    on_saved: move |_| {
                        editing.set(None);
                        entries.restart();
                        notify_change(&mut guard);
                    },
                    on_cancel: move |_| editing.set(None),
                    error_msg: error_msg,
                }
            }

            // Table
            match &*entries.read() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    EmptyState {
                        icon: rsx! { Icon { width: 32, height: 32, icon: LdGlobe } },
                        title: "No DDNS Entries".to_string(),
                        subtitle: "Add a DDNS entry to keep your DNS records in sync with your WAN IP.".to_string(),
                    }
                },
                Some(Ok(list)) => {
                    let mut sorted = list.clone();
                    sorted.sort_by_key(|e| e.id);
                    rsx! {
                        DataTable {
                            thead {
                                tr {
                                    th { class: TH_CLASS, "Provider" }
                                    th { class: TH_CLASS, "Hostname" }
                                    th { class: TH_CLASS, "Current IP" }
                                    th { class: TH_CLASS, "Last Update" }
                                    th { class: TH_CLASS, "Status" }
                                    th { class: "{TH_CLASS} text-right", "Actions" }
                                }
                            }
                            tbody {
                                for entry in sorted {
                                    {
                                        let entry_id = entry.id;
                                        let entry_edit = entry.clone();
                                        let status = status_map.get(&entry.id).cloned();
                                        let is_updating = updating_id() == Some(entry.id);

                                        let status_badge = if !entry.enabled {
                                            ("Disabled", Color::Slate)
                                        } else if status.as_ref().and_then(|s| s.last_error.as_ref()).is_some() {
                                            ("Error", Color::Red)
                                        } else if status.as_ref().and_then(|s| s.current_ip.as_ref()).is_some() {
                                            ("Active", Color::Emerald)
                                        } else {
                                            ("Pending", Color::Amber)
                                        };

                                        let current_ip = status.as_ref().and_then(|s| s.current_ip.clone()).unwrap_or("-".to_string());
                                        let last_update = status.as_ref().and_then(|s| s.last_update.as_deref().map(format_time)).unwrap_or("-".to_string());
                                        let error_tooltip = status.as_ref().and_then(|s| s.last_error.clone()).unwrap_or_default();

                                        rsx! {
                                            tr { class: TR_CLASS,
                                                td { class: TD_CLASS,
                                                    Badge { color: Color::Slate, label: provider_label(&entry.provider).to_string() }
                                                }
                                                td { class: TD_CLASS,
                                                    span { class: "font-mono text-slate-200", "{entry.hostname}" }
                                                }
                                                td { class: TD_CLASS,
                                                    span { class: "font-mono text-slate-400", "{current_ip}" }
                                                }
                                                td { class: TD_CLASS,
                                                    span { class: "text-slate-500", "{last_update}" }
                                                }
                                                td { class: TD_CLASS,
                                                    span { title: "{error_tooltip}",
                                                        Badge { color: status_badge.1, label: status_badge.0.to_string() }
                                                    }
                                                }
                                                td { class: "{TD_CLASS} text-right",
                                                    div { class: "flex items-center justify-end gap-1",
                                                        // Force update
                                                        button {
                                                            class: "w-7 h-7 rounded-lg hover:bg-blue-500/10 flex items-center justify-center transition-colors",
                                                            title: "Update now",
                                                            disabled: is_updating,
                                                            onclick: move |_| {
                                                                updating_id.set(Some(entry_id));
                                                                spawn(async move {
                                                                    let _ = api_client::post::<(), serde_json::Value>(&format!("/ddns/{}/update-now", entry_id), &()).await;
                                                                    updating_id.set(None);
                                                                    statuses.restart();
                                                                });
                                                            },
                                                            Icon {
                                                                width: 13, height: 13, icon: LdRefreshCw,
                                                                class: if is_updating { "text-blue-400 animate-spin" } else { "text-blue-400" },
                                                            }
                                                        }
                                                        // Toggle
                                                        button {
                                                            class: "w-7 h-7 rounded-lg hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                            title: if entry.enabled { "Disable" } else { "Enable" },
                                                            onclick: move |_| {
                                                                spawn(async move {
                                                                    let _ = api_client::post::<(), serde_json::Value>(&format!("/ddns/{}/toggle", entry_id), &()).await;
                                                                    entries.restart();
                                                                });
                                                            },
                                                            if entry.enabled {
                                                                Icon { width: 13, height: 13, icon: LdToggleRight, class: "text-emerald-400" }
                                                            } else {
                                                                Icon { width: 13, height: 13, icon: LdToggleLeft, class: "text-slate-600" }
                                                            }
                                                        }
                                                        // Edit
                                                        button {
                                                            class: "w-7 h-7 rounded-lg hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                            title: "Edit",
                                                            onclick: move |_| editing.set(Some((true, entry_edit.clone()))),
                                                            Icon { width: 13, height: 13, icon: LdPencil, class: "text-slate-400" }
                                                        }
                                                        // Delete
                                                        button {
                                                            class: "w-7 h-7 rounded-lg hover:bg-red-500/10 flex items-center justify-center transition-colors",
                                                            title: "Delete",
                                                            onclick: move |_| confirm_delete.set(Some(entry_id)),
                                                            Icon { width: 13, height: 13, icon: LdTrash2, class: "text-red-400" }
                                                        }
                                                    }
                                                }
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

            // Delete confirmation
            if let Some(id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete DDNS Entry".to_string(),
                    message: "Are you sure? DNS records will no longer be updated automatically.".to_string(),
                    on_confirm: move |_| {
                        spawn(async move {
                            let _ = api_client::delete(&format!("/ddns/{}", id)).await;
                            confirm_delete.set(None);
                            entries.restart();
                            notify_change(&mut guard);
                        });
                    },
                    on_cancel: move |_| confirm_delete.set(None),
                }
            }
        }
    }
}

fn provider_label(p: &DdnsProvider) -> &'static str {
    match p {
        DdnsProvider::Cloudflare => "Cloudflare",
        DdnsProvider::NoIp => "No-IP",
        DdnsProvider::DuckDns => "DuckDNS",
        DdnsProvider::Dynu => "Dynu",
        DdnsProvider::Custom => "Custom",
    }
}

fn format_time(ts: &str) -> String {
    if let Some(t) = ts.get(..19) {
        t.replace('T', " ")
    } else {
        ts.to_string()
    }
}

// === Form ===

#[component]
fn DdnsForm(
    is_edit: bool,
    entry: DdnsEntry,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    mut error_msg: Signal<Option<String>>,
) -> Element {
    let entry_id = entry.id;
    let mut provider = use_signal(move || entry.provider.clone());
    let mut hostname = use_signal(move || entry.hostname.clone());
    let mut username = use_signal(move || entry.username.clone());
    let mut token = use_signal(move || entry.token.clone());
    let mut custom_url = use_signal(move || entry.custom_url.clone());
    let mut interval = use_signal(move || (entry.interval_secs / 60).max(1).to_string());
    let mut enabled = use_signal(move || entry.enabled);
    let mut submitting = use_signal(|| false);

    let prov = provider();
    let show_username = matches!(
        prov,
        DdnsProvider::Cloudflare | DdnsProvider::NoIp | DdnsProvider::Dynu
    );
    let username_label = match prov {
        DdnsProvider::Cloudflare => "Zone ID",
        _ => "Username",
    };
    let token_label = match prov {
        DdnsProvider::Cloudflare => "API Token",
        DdnsProvider::DuckDns => "Token",
        _ => "Password / Token",
    };

    rsx! {
        FormCard {
            form {
                onsubmit: move |e| {
                    e.prevent_default();
                    if submitting() {
                        return;
                    }
                    submitting.set(true);
                    error_msg.set(None);
                    spawn(async move {
                        let interval_mins: u64 = interval().parse().unwrap_or(5);
                        let body = DdnsEntry {
                            id: entry_id,
                            provider: provider(),
                            hostname: hostname(),
                            username: username(),
                            token: token(),
                            custom_url: custom_url(),
                            interval_secs: interval_mins * 60,
                            enabled: enabled(),
                        };
                        let result = if is_edit {
                            api_client::put::<DdnsEntry, DdnsEntry>(
                                &format!("/ddns/{}", entry_id),
                                &body,
                            )
                            .await
                        } else {
                            api_client::post::<DdnsEntry, DdnsEntry>("/ddns", &body).await
                        };
                        match result {
                            Ok(_) => on_saved.call(()),
                            Err(e) => error_msg.set(Some(e)),
                        }
                        submitting.set(false);
                    });
                },
                class: "space-y-4",

                h3 {
                    class: "text-sm font-semibold text-white mb-3",
                    if is_edit {
                        "Edit DDNS Entry"
                    } else {
                        "New DDNS Entry"
                    }
                }

                div { class: "grid grid-cols-2 gap-4",
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Provider" }
                        select {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white",
                            value: provider_value(&provider()),
                            onchange: move |e| provider.set(parse_provider(&e.value())),
                            option { value: "cloudflare", "Cloudflare" }
                            option { value: "duckdns", "DuckDNS" }
                            option { value: "noip", "No-IP" }
                            option { value: "dynu", "Dynu" }
                            option { value: "custom", "Custom URL" }
                        }
                    }
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Hostname" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                            r#type: "text",
                            placeholder: "myhost.example.com",
                            value: "{hostname}",
                            oninput: move |e| hostname.set(e.value()),
                        }
                    }
                }

                div { class: "grid grid-cols-2 gap-4",
                    if show_username {
                        div {
                            label { class: "block text-xs font-medium text-slate-400 mb-1.5", "{username_label}" }
                            input {
                                class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                                r#type: "text",
                                value: "{username}",
                                oninput: move |e| username.set(e.value()),
                            }
                        }
                    }
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "{token_label}" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                            r#type: "password",
                            value: "{token}",
                            oninput: move |e| token.set(e.value()),
                        }
                    }
                }

                if prov == DdnsProvider::Custom {
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Update URL" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                            r#type: "text",
                            placeholder: "https://example.com/update?ip=__IP__&host=__HOST__",
                            value: "{custom_url}",
                            oninput: move |e| custom_url.set(e.value()),
                        }
                    }
                }

                div { class: "grid grid-cols-2 gap-4",
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Check Interval (minutes)" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white",
                            r#type: "number",
                            min: "1",
                            value: "{interval}",
                            oninput: move |e| interval.set(e.value()),
                        }
                    }
                    div { class: "flex items-center gap-2 pt-6",
                        input {
                            r#type: "checkbox",
                            class: "rounded bg-slate-800 border-slate-600",
                            checked: enabled(),
                            onchange: move |e| enabled.set(e.checked()),
                        }
                        label { class: "text-sm text-slate-300", "Enabled" }
                    }
                }

                div { class: "flex gap-2 pt-2",
                    SubmitBtn {
                        color: Color::Blue,
                        label: if submitting() {
                            "Saving...".to_string()
                        } else if is_edit {
                            "Update".to_string()
                        } else {
                            "Create".to_string()
                        },
                        disabled: submitting() || hostname().is_empty() || token().is_empty(),
                        onclick: move |_| {},
                    }
                    Btn {
                        color: Color::Slate,
                        label: "Cancel".to_string(),
                        onclick: move |_| on_cancel.call(()),
                    }
                }
            }
        }
    }
}

fn provider_value(p: &DdnsProvider) -> &'static str {
    match p {
        DdnsProvider::Cloudflare => "cloudflare",
        DdnsProvider::NoIp => "noip",
        DdnsProvider::DuckDns => "duckdns",
        DdnsProvider::Dynu => "dynu",
        DdnsProvider::Custom => "custom",
    }
}

fn parse_provider(s: &str) -> DdnsProvider {
    match s {
        "cloudflare" => DdnsProvider::Cloudflare,
        "noip" => DdnsProvider::NoIp,
        "duckdns" => DdnsProvider::DuckDns,
        "dynu" => DdnsProvider::Dynu,
        _ => DdnsProvider::Custom,
    }
}
