use super::{ConfirmModal, use_change_guard, use_refresh_trigger, notify_change};
use super::ui::*;
use crate::api_client;
use crate::models::*;
use crate::ws_client::use_ws_events;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn L4Proxy() -> Element {
    let mut rules = use_resource(|| async { api_client::get::<Vec<L4ProxyRule>>("/l4proxy/rules").await });
    let mut editing = use_signal(|| None::<(bool, L4ProxyRule)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut confirm_toggle = use_signal(|| None::<(u32, bool)>);
    let mut guard = use_change_guard();

    let ws = use_ws_events();
    let refresh = use_refresh_trigger();
    let mut prev = use_signal(|| (refresh(), ws.l4proxy()));
    use_effect(move || {
        let current = (refresh(), ws.l4proxy());
        if current != prev() {
            prev.set(current);
            rules.restart();
        }
    });

    rsx! {
        div {
            PageHeader {
                title: "L4 Proxy".to_string(),
                subtitle: "Layer 4 load balancer with eBPF DNAT/SNAT".to_string(),
                Btn {
                    color: Color::Blue,
                    label: if editing().is_some() { "Cancel".to_string() } else { "+ New Proxy Rule".to_string() },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((false, L4ProxyRule {
                                id: 0,
                                name: String::new(),
                                protocol: L4Protocol::TCP,
                                listen_address: "0.0.0.0".to_string(),
                                listen_port: 0,
                                upstream_targets: vec![UpstreamTarget { address: String::new(), port: 0, weight: 1 }],
                                load_balance: LoadBalanceMode::RoundRobin,
                                enabled: true,
                            })));
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

            if let Some((is_edit, rule)) = editing() {
                L4ProxyForm {
                    key: "{rule.id}",
                    is_edit: is_edit,
                    editing: rule,
                    on_saved: move |_| {
                        editing.set(None);
                        rules.restart();
                        notify_change(&mut guard);
                    }
                }
            }

            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Proxy Rule".to_string(),
                    message: format!("Are you sure you want to delete proxy rule #{}? This action cannot be undone.", del_id),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/l4proxy/rules/{}", del_id)).await {
                                Ok(_) => {
                                    rules.restart();
                                    notify_change(&mut guard);
                                }
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete.set(None); },
                }
            }

            if let Some((tog_id, tog_enabled)) = confirm_toggle() {
                ConfirmModal {
                    title: if tog_enabled { "Disable Proxy Rule".to_string() } else { "Enable Proxy Rule".to_string() },
                    message: format!(
                        "Are you sure you want to {} proxy rule #{}?",
                        if tog_enabled { "disable" } else { "enable" },
                        tog_id
                    ),
                    confirm_label: if tog_enabled { "Disable".to_string() } else { "Enable".to_string() },
                    danger: tog_enabled,
                    on_confirm: move |_| {
                        confirm_toggle.set(None);
                        spawn(async move {
                            match api_client::post::<(), L4ProxyRule>(&format!("/l4proxy/rules/{}/toggle", tog_id), &()).await {
                                Ok(_) => { rules.restart(); notify_change(&mut guard); },
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_toggle.set(None); },
                }
            }

            DataTable {
                thead { class: "bg-slate-900/80",
                    tr {
                        th { class: TH_CLASS, "Name" }
                        th { class: TH_CLASS, "Protocol" }
                        th { class: TH_CLASS, "Listen" }
                        th { class: TH_CLASS, "Upstreams" }
                        th { class: TH_CLASS, "Balance" }
                        th { class: TH_CLASS, "Status" }
                        th { class: TH_CLASS, "" }
                    }
                }
                tbody {
                    match &*rules.read() {
                        Some(Ok(list)) if !list.is_empty() => rsx! {
                            for rule in list.iter() {
                                tr { class: TR_CLASS,
                                    key: "{rule.id}",
                                    td { class: "{TD_CLASS} text-slate-200 font-medium",
                                        "{rule.name}"
                                    }
                                    td { class: TD_CLASS,
                                        Badge {
                                            color: match rule.protocol {
                                                L4Protocol::TCP => Color::Blue,
                                                L4Protocol::UDP => Color::Violet,
                                            },
                                            label: match rule.protocol {
                                                L4Protocol::TCP => "TCP".to_string(),
                                                L4Protocol::UDP => "UDP".to_string(),
                                            },
                                        }
                                    }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono text-xs",
                                        "{rule.listen_address}:{rule.listen_port}"
                                    }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono text-xs",
                                        {format_upstreams(&rule.upstream_targets)}
                                    }
                                    td { class: TD_CLASS,
                                        Badge {
                                            color: Color::Slate,
                                            label: match rule.load_balance {
                                                LoadBalanceMode::RoundRobin => "Round Robin".to_string(),
                                                LoadBalanceMode::IpHash => "IP Hash".to_string(),
                                            },
                                        }
                                    }
                                    td { class: TD_CLASS,
                                        {
                                            let id = rule.id;
                                            let enabled = rule.enabled;
                                            rsx! {
                                                button {
                                                    class: if enabled {
                                                        "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20 transition-colors cursor-pointer"
                                                    } else {
                                                        "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-500 border border-slate-500/20 hover:bg-slate-500/20 transition-colors cursor-pointer"
                                                    },
                                                    onclick: move |_| {
                                                        confirm_toggle.set(Some((id, enabled)));
                                                    },
                                                    if enabled { "Enabled" } else { "Disabled" }
                                                }
                                            }
                                        }
                                    }
                                    td { class: TD_CLASS,
                                        {
                                            let rule_clone = rule.clone();
                                            let id = rule.id;
                                            rsx! {
                                                div { class: "flex items-center gap-1",
                                                    EditBtn {
                                                        onclick: move |_| {
                                                            editing.set(Some((true, rule_clone.clone())));
                                                        },
                                                    }
                                                    DeleteBtn {
                                                        onclick: move |_| {
                                                            confirm_delete.set(Some(id));
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Ok(_)) => rsx! {
                            TableEmpty { colspan: 7, message: "No L4 proxy rules configured".to_string() }
                        },
                        Some(Err(e)) => rsx! {
                            TableError { colspan: 7, message: format!("Failed to load proxy rules: {e}") }
                        },
                        None => rsx! {
                            TableLoading { colspan: 7 }
                        },
                    }
                }
            }
        }
    }
}

fn format_upstreams(targets: &[UpstreamTarget]) -> String {
    if targets.len() == 1 {
        format!("{}:{}", targets[0].address, targets[0].port)
    } else {
        format!("{} targets", targets.len())
    }
}

// === L4 Proxy Form ===

#[component]
fn L4ProxyForm(is_edit: bool, editing: L4ProxyRule, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut name = use_signal(|| editing.name.clone());
    let mut protocol = use_signal(|| match editing.protocol {
        L4Protocol::TCP => "TCP".to_string(),
        L4Protocol::UDP => "UDP".to_string(),
    });
    let mut listen_address = use_signal(|| editing.listen_address.clone());
    let mut listen_port = use_signal(|| if editing.listen_port == 0 { String::new() } else { editing.listen_port.to_string() });
    let mut load_balance = use_signal(|| match editing.load_balance {
        LoadBalanceMode::RoundRobin => "RoundRobin".to_string(),
        LoadBalanceMode::IpHash => "IpHash".to_string(),
    });
    let mut targets = use_signal(|| editing.upstream_targets.clone());
    let editing_enabled = editing.enabled;
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        if name().trim().is_empty() {
            error.set(Some("Name is required".to_string()));
            return;
        }
        let port: u16 = match listen_port().parse() {
            Ok(p) if p > 0 => p,
            _ => {
                error.set(Some("Listen port must be a valid number (1-65535)".to_string()));
                return;
            }
        };
        let current_targets = targets();
        if current_targets.is_empty() {
            error.set(Some("At least one upstream target is required".to_string()));
            return;
        }
        for (i, t) in current_targets.iter().enumerate() {
            if t.address.trim().is_empty() {
                error.set(Some(format!("Upstream #{}: address is required", i + 1)));
                return;
            }
            if t.port == 0 {
                error.set(Some(format!("Upstream #{}: port must be > 0", i + 1)));
                return;
            }
        }

        submitting.set(true);
        error.set(None);

        let rule = L4ProxyRule {
            id: edit_id,
            name: name().trim().to_string(),
            protocol: match protocol().as_str() {
                "UDP" => L4Protocol::UDP,
                _ => L4Protocol::TCP,
            },
            listen_address: listen_address(),
            listen_port: port,
            upstream_targets: current_targets,
            load_balance: match load_balance().as_str() {
                "IpHash" => LoadBalanceMode::IpHash,
                _ => LoadBalanceMode::RoundRobin,
            },
            enabled: if is_edit { editing_enabled } else { true },
        };

        spawn(async move {
            let result = if is_edit {
                api_client::put::<L4ProxyRule, L4ProxyRule>(&format!("/l4proxy/rules/{}", edit_id), &rule).await
            } else {
                api_client::post::<L4ProxyRule, L4ProxyRule>("/l4proxy/rules", &rule).await
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        FormCard {
            h3 { class: "text-sm font-semibold text-white mb-4",
                if is_edit { "Edit Proxy Rule" } else { "Create Proxy Rule" }
            }
            if let Some(err) = error() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error.set(None),
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4 mb-4",
                FormField { label: "Name".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "e.g. Web Backend",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                FormField { label: "Protocol".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{protocol}", onchange: move |e| protocol.set(e.value()),
                        option { value: "TCP", "TCP" }
                        option { value: "UDP", "UDP" }
                    }
                }
                FormField { label: "Listen Address".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "0.0.0.0",
                        value: "{listen_address}",
                        oninput: move |e| listen_address.set(e.value()),
                    }
                }
                FormField { label: "Listen Port".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", placeholder: "e.g. 8080",
                        value: "{listen_port}",
                        oninput: move |e| listen_port.set(e.value()),
                    }
                }
                FormField { label: "Load Balance".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{load_balance}", onchange: move |e| load_balance.set(e.value()),
                        option { value: "RoundRobin", "Round Robin" }
                        option { value: "IpHash", "IP Hash" }
                    }
                }
            }

            // Upstream targets
            div { class: "mb-4",
                div { class: "flex items-center justify-between mb-2",
                    p { class: "text-xs font-semibold text-slate-400 uppercase tracking-wide", "Upstream Targets" }
                    button {
                        class: "text-xs text-blue-400 hover:text-blue-300 transition-colors cursor-pointer",
                        onclick: move |_| {
                            let mut current = targets();
                            current.push(UpstreamTarget { address: String::new(), port: 0, weight: 1 });
                            targets.set(current);
                        },
                        "+ Add Target"
                    }
                }
                div { class: "flex items-center gap-2 mb-1",
                    p { class: "flex-1 min-w-0 text-[11px] text-slate-500", "Address" }
                    p { class: "w-24 shrink-0 text-[11px] text-slate-500", "Port" }
                    p { class: "w-20 shrink-0 text-[11px] text-slate-500", "Weight" }
                    if targets().len() > 1 {
                        div { class: "w-7 shrink-0" }
                    }
                }
                for (idx, _target) in targets().iter().enumerate() {
                    {
                        let current_targets = targets();
                        let t = &current_targets[idx];
                        let addr_val = t.address.clone();
                        let port_val = if t.port == 0 { String::new() } else { t.port.to_string() };
                        let weight_val = t.weight.to_string();
                        let input_base = "bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";
                        rsx! {
                            div { class: "flex items-center gap-2 mb-2",
                                key: "{idx}",
                                input {
                                    class: "{input_base} flex-1 min-w-0",
                                    r#type: "text",
                                    placeholder: "Address (e.g. 10.0.0.1)",
                                    value: "{addr_val}",
                                    oninput: move |e| {
                                        let mut current = targets();
                                        if let Some(t) = current.get_mut(idx) {
                                            t.address = e.value();
                                        }
                                        targets.set(current);
                                    },
                                }
                                input {
                                    class: "{input_base} w-24 shrink-0",
                                    r#type: "number",
                                    placeholder: "Port",
                                    value: "{port_val}",
                                    oninput: move |e| {
                                        let mut current = targets();
                                        if let Some(t) = current.get_mut(idx) {
                                            t.port = e.value().parse().unwrap_or(0);
                                        }
                                        targets.set(current);
                                    },
                                }
                                input {
                                    class: "{input_base} w-20 shrink-0",
                                    r#type: "number",
                                    placeholder: "Weight",
                                    value: "{weight_val}",
                                    oninput: move |e| {
                                        let mut current = targets();
                                        if let Some(t) = current.get_mut(idx) {
                                            t.weight = e.value().parse().unwrap_or(1);
                                        }
                                        targets.set(current);
                                    },
                                }
                                if targets().len() > 1 {
                                    button {
                                        class: "w-7 h-7 rounded-lg bg-red-500/10 hover:bg-red-500/20 flex items-center justify-center text-red-400 transition-colors cursor-pointer shrink-0",
                                        onclick: move |_| {
                                            let mut current = targets();
                                            if idx < current.len() {
                                                current.remove(idx);
                                                targets.set(current);
                                            }
                                        },
                                        Icon { width: 12, height: 12, icon: LdX }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "text-[11px] text-slate-600", "Weight determines traffic distribution ratio. Higher weight = more traffic." }
            }

            SubmitBtn {
                color: Color::Blue,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Creating...".to_string() }
                } else {
                    if is_edit { "Save Rule".to_string() } else { "Create Rule".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}
