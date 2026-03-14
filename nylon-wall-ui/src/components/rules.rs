use super::ConfirmModal;
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;

#[component]
pub fn Rules() -> Element {
    let mut rules = use_resource(|| async { api_client::get::<Vec<FirewallRule>>("/rules").await });
    let mut editing = use_signal(|| None::<(bool, FirewallRule)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(u32, String)>);

    rsx! {
        div {
            PageHeader {
                title: "Firewall Rules".to_string(),
                subtitle: "Manage ingress and egress filtering rules".to_string(),
                Btn {
                    color: Color::Blue,
                    label: if editing().is_some() { "Cancel".to_string() } else { "+ New Rule".to_string() },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((false, FirewallRule {
                                id: 0, name: String::new(), priority: 100,
                                direction: Direction::Ingress, enabled: true,
                                src_ip: None, dst_ip: None, src_port: None, dst_port: None,
                                protocol: None, interface: None, action: RuleAction::Allow,
                                rate_limit_pps: None, hit_count: 0, created_at: 0, updated_at: 0,
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
                RuleForm {
                    key: "{rule.id}",
                    is_edit: is_edit,
                    editing: rule,
                    on_saved: move |_| {
                        editing.set(None);
                        rules.restart();
                    }
                }
            }

            if let Some((del_id, del_name)) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Rule".to_string(),
                    message: format!("Are you sure you want to delete rule \"{}\"? This action cannot be undone.", del_name),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/rules/{}", del_id)).await {
                                Ok(_) => rules.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete.set(None); },
                }
            }

            DataTable {
                thead { class: "bg-slate-900/80",
                    tr {
                        th { class: TH_CLASS, "Name" }
                        th { class: TH_CLASS, "Direction" }
                        th { class: TH_CLASS, "Protocol" }
                        th { class: TH_CLASS, "Source" }
                        th { class: TH_CLASS, "Destination" }
                        th { class: TH_CLASS, "Action" }
                        th { class: TH_CLASS, "Hits" }
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
                                    td { class: "{TD_CLASS} text-slate-300 font-medium", "{rule.name}" }
                                    td { class: TD_CLASS,
                                        Badge {
                                            color: match rule.direction {
                                                Direction::Ingress => Color::Blue,
                                                Direction::Egress => Color::Violet,
                                            },
                                            label: match rule.direction {
                                                Direction::Ingress => "IN".to_string(),
                                                Direction::Egress => "OUT".to_string(),
                                            },
                                        }
                                    }
                                    td { class: "{TD_CLASS} text-slate-400", {rule.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono", {rule.src_ip.clone().unwrap_or("*".to_string())} }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono", {rule.dst_ip.clone().unwrap_or("*".to_string())} }
                                    td { class: TD_CLASS,
                                        Badge {
                                            color: match rule.action {
                                                RuleAction::Allow => Color::Emerald,
                                                RuleAction::Drop => Color::Red,
                                                RuleAction::Log => Color::Amber,
                                                RuleAction::RateLimit => Color::Cyan,
                                            },
                                            label: match rule.action {
                                                RuleAction::Allow => "ALLOW".to_string(),
                                                RuleAction::Drop => "DROP".to_string(),
                                                RuleAction::Log => "LOG".to_string(),
                                                RuleAction::RateLimit => "RATE LIMIT".to_string(),
                                            },
                                        }
                                    }
                                    td { class: "{TD_CLASS} text-cyan-400 font-mono", "{rule.hit_count}" }
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
                                                        spawn(async move {
                                                            match api_client::post::<(), FirewallRule>(&format!("/rules/{}/toggle", id), &()).await {
                                                                Ok(_) => rules.restart(),
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
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
                                            let name = rule.name.clone();
                                            rsx! {
                                                div { class: "flex items-center gap-1",
                                                    EditBtn {
                                                        onclick: move |_| {
                                                            editing.set(Some((true, rule_clone.clone())));
                                                        },
                                                    }
                                                    DeleteBtn {
                                                        onclick: move |_| {
                                                            confirm_delete.set(Some((id, name.clone())));
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
                            TableEmpty { colspan: 9, message: "No firewall rules configured".to_string() }
                        },
                        Some(Err(e)) => rsx! {
                            TableError { colspan: 9, message: format!("Failed to load rules: {e}") }
                        },
                        None => rsx! {
                            TableLoading { colspan: 9 }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn RuleForm(is_edit: bool, editing: FirewallRule, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut name = use_signal(|| editing.name.clone());
    let mut direction = use_signal(|| match editing.direction {
        Direction::Egress => "Egress".to_string(),
        Direction::Ingress => "Ingress".to_string(),
    });
    let mut protocol = use_signal(|| match editing.protocol {
        Some(Protocol::TCP) => "TCP".to_string(),
        Some(Protocol::UDP) => "UDP".to_string(),
        Some(Protocol::ICMP) => "ICMP".to_string(),
        _ => "Any".to_string(),
    });
    let mut src_ip = use_signal(|| editing.src_ip.clone().unwrap_or_default());
    let mut dst_ip = use_signal(|| editing.dst_ip.clone().unwrap_or_default());
    let mut dst_port = use_signal(|| {
        editing.dst_port.as_ref().map(|p| {
            if p.start == p.end { p.start.to_string() } else { format!("{}-{}", p.start, p.end) }
        }).unwrap_or_default()
    });
    let mut action = use_signal(|| match editing.action {
        RuleAction::Drop => "Drop".to_string(),
        RuleAction::Log => "Log".to_string(),
        _ => "Allow".to_string(),
    });
    let mut priority = use_signal(|| editing.priority.to_string());
    let editing_enabled = editing.enabled;
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let rule = FirewallRule {
            id: edit_id,
            name: name(),
            priority: priority().parse().unwrap_or(100),
            direction: match direction().as_str() {
                "Egress" => Direction::Egress,
                _ => Direction::Ingress,
            },
            enabled: if is_edit { editing_enabled } else { true },
            src_ip: if src_ip().is_empty() {
                None
            } else {
                Some(src_ip())
            },
            dst_ip: if dst_ip().is_empty() {
                None
            } else {
                Some(dst_ip())
            },
            src_port: None,
            dst_port: if dst_port().is_empty() {
                None
            } else {
                dst_port().parse::<u16>().ok().map(PortRange::single)
            },
            protocol: match protocol().as_str() {
                "TCP" => Some(Protocol::TCP),
                "UDP" => Some(Protocol::UDP),
                "ICMP" => Some(Protocol::ICMP),
                _ => None,
            },
            interface: None,
            action: match action().as_str() {
                "Drop" => RuleAction::Drop,
                "Log" => RuleAction::Log,
                _ => RuleAction::Allow,
            },
            rate_limit_pps: None,
            hit_count: 0,
            created_at: 0,
            updated_at: 0,
        };
        spawn(async move {
            let result = if is_edit {
                api_client::put::<FirewallRule, FirewallRule>(&format!("/rules/{}", edit_id), &rule).await
            } else {
                api_client::post::<FirewallRule, FirewallRule>("/rules", &rule).await
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
                if is_edit { "Edit Rule" } else { "Create New Rule" }
            }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                FormField { label: "Name".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "Rule name", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                FormField { label: "Priority".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", value: "{priority}",
                        oninput: move |e| priority.set(e.value()),
                    }
                }
                FormField { label: "Direction".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{direction}", onchange: move |e| direction.set(e.value()),
                        option { value: "Ingress", "Ingress" }
                        option { value: "Egress", "Egress" }
                    }
                }
                FormField { label: "Protocol".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{protocol}", onchange: move |e| protocol.set(e.value()),
                        option { value: "Any", "Any" }
                        option { value: "TCP", "TCP" }
                        option { value: "UDP", "UDP" }
                        option { value: "ICMP", "ICMP" }
                    }
                }
                FormField { label: "Source IP".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{src_ip}",
                        oninput: move |e| src_ip.set(e.value()),
                    }
                }
                FormField { label: "Destination IP".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "10.0.0.0/8", value: "{dst_ip}",
                        oninput: move |e| dst_ip.set(e.value()),
                    }
                }
                FormField { label: "Dest Port".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "80", value: "{dst_port}",
                        oninput: move |e| dst_port.set(e.value()),
                    }
                }
                FormField { label: "Action".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{action}", onchange: move |e| action.set(e.value()),
                        option { value: "Allow", "Allow" }
                        option { value: "Drop", "Drop" }
                        option { value: "Log", "Log" }
                    }
                }
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
