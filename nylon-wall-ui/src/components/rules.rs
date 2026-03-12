use dioxus::prelude::*;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Rules() -> Element {
    let mut rules = use_resource(|| async {
        api_client::get::<Vec<FirewallRule>>("/rules").await
    });
    let mut show_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    let toggle_rule = move |id: u32| {
        spawn(async move {
            match api_client::post::<(), FirewallRule>(&format!("/rules/{}/toggle", id), &()).await {
                Ok(_) => rules.restart(),
                Err(e) => error_msg.set(Some(e)),
            }
        });
    };

    let delete_rule = move |id: u32| {
        spawn(async move {
            match api_client::delete(&format!("/rules/{}", id)).await {
                Ok(_) => rules.restart(),
                Err(e) => error_msg.set(Some(e)),
            }
        });
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                h1 { "Firewall Rules" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ New Rule" }
                }
            }

            if let Some(err) = error_msg() {
                div { class: "alert alert-error",
                    "{err}"
                    button { class: "btn-close", onclick: move |_| error_msg.set(None), "\u{00d7}" }
                }
            }

            if show_form() {
                RuleForm {
                    on_saved: move |_| {
                        show_form.set(false);
                        rules.restart();
                    }
                }
            }

            match &*rules.read() {
                Some(Ok(rule_list)) => rsx! {
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "ID" }
                                th { "Name" }
                                th { "Direction" }
                                th { "Protocol" }
                                th { "Source" }
                                th { "Destination" }
                                th { "Action" }
                                th { "Hits" }
                                th { "Status" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            for rule in rule_list.iter() {
                                tr { key: "{rule.id}",
                                    td { "{rule.id}" }
                                    td { "{rule.name}" }
                                    td {
                                        span { class: "badge",
                                            match rule.direction {
                                                Direction::Ingress => "IN",
                                                Direction::Egress => "OUT",
                                            }
                                        }
                                    }
                                    td { {rule.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                    td { {rule.src_ip.clone().unwrap_or("Any".to_string())} }
                                    td { {rule.dst_ip.clone().unwrap_or("Any".to_string())} }
                                    td {
                                        span {
                                            class: match rule.action {
                                                RuleAction::Allow => "badge badge-success",
                                                RuleAction::Drop => "badge badge-error",
                                                RuleAction::Log => "badge badge-warning",
                                                RuleAction::RateLimit => "badge badge-warning",
                                            },
                                            match rule.action {
                                                RuleAction::Allow => "ALLOW",
                                                RuleAction::Drop => "DROP",
                                                RuleAction::Log => "LOG",
                                                RuleAction::RateLimit => "RATE LIMIT",
                                            }
                                        }
                                    }
                                    td { "{rule.hit_count}" }
                                    td {
                                        {
                                            let id = rule.id;
                                            rsx! {
                                                button {
                                                    class: if rule.enabled { "btn btn-sm btn-success" } else { "btn btn-sm btn-muted" },
                                                    onclick: move |_| toggle_rule(id),
                                                    if rule.enabled { "Enabled" } else { "Disabled" }
                                                }
                                            }
                                        }
                                    }
                                    td {
                                        {
                                            let id = rule.id;
                                            rsx! {
                                                button {
                                                    class: "btn btn-sm btn-danger",
                                                    onclick: move |_| delete_rule(id),
                                                    "Delete"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if rule_list.is_empty() {
                        p { class: "empty-state", "No firewall rules configured. Click '+ New Rule' to create one." }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Failed to load rules: {e}" } },
                None => rsx! { p { "Loading rules..." } },
            }
        }
    }
}

#[component]
fn RuleForm(on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(|| String::new());
    let mut direction = use_signal(|| "Ingress".to_string());
    let mut protocol = use_signal(|| "Any".to_string());
    let mut src_ip = use_signal(|| String::new());
    let mut dst_ip = use_signal(|| String::new());
    let mut dst_port = use_signal(|| String::new());
    let mut action = use_signal(|| "Allow".to_string());
    let mut priority = use_signal(|| "100".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let rule = FirewallRule {
            id: 0, // server assigns
            name: name(),
            priority: priority().parse().unwrap_or(100),
            direction: match direction().as_str() {
                "Egress" => Direction::Egress,
                _ => Direction::Ingress,
            },
            enabled: true,
            src_ip: if src_ip().is_empty() { None } else { Some(src_ip()) },
            dst_ip: if dst_ip().is_empty() { None } else { Some(dst_ip()) },
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
            match api_client::post::<FirewallRule, FirewallRule>("/rules", &rule).await {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "card form-card",
            h3 { "Create New Rule" }
            if let Some(err) = error() {
                div { class: "alert alert-error", "{err}" }
            }
            div { class: "form-grid",
                div { class: "form-group",
                    label { "Name" }
                    input {
                        r#type: "text",
                        placeholder: "Rule name",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Priority" }
                    input {
                        r#type: "number",
                        value: "{priority}",
                        oninput: move |e| priority.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Direction" }
                    select {
                        value: "{direction}",
                        onchange: move |e| direction.set(e.value()),
                        option { value: "Ingress", "Ingress" }
                        option { value: "Egress", "Egress" }
                    }
                }
                div { class: "form-group",
                    label { "Protocol" }
                    select {
                        value: "{protocol}",
                        onchange: move |e| protocol.set(e.value()),
                        option { value: "Any", "Any" }
                        option { value: "TCP", "TCP" }
                        option { value: "UDP", "UDP" }
                        option { value: "ICMP", "ICMP" }
                    }
                }
                div { class: "form-group",
                    label { "Source IP (CIDR)" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 192.168.1.0/24",
                        value: "{src_ip}",
                        oninput: move |e| src_ip.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Destination IP (CIDR)" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 10.0.0.0/8",
                        value: "{dst_ip}",
                        oninput: move |e| dst_ip.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Destination Port" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 80 or 8000-9000",
                        value: "{dst_port}",
                        oninput: move |e| dst_port.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Action" }
                    select {
                        value: "{action}",
                        onchange: move |e| action.set(e.value()),
                        option { value: "Allow", "Allow" }
                        option { value: "Drop", "Drop" }
                        option { value: "Log", "Log" }
                    }
                }
            }
            div { class: "form-actions",
                button {
                    class: "btn btn-primary",
                    disabled: submitting(),
                    onclick: on_submit,
                    if submitting() { "Creating..." } else { "Create Rule" }
                }
            }
        }
    }
}
