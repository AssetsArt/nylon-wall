use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Rules() -> Element {
    let mut rules = use_resource(|| async {
        api_client::get::<Vec<FirewallRule>>("/rules").await
    });
    let mut show_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "Firewall Rules" }
                    p { class: "text-sm text-slate-400 mt-1", "Manage ingress and egress filtering rules" }
                }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ New Rule" }
                }
            }

            if let Some(err) = error_msg() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 flex items-center justify-between",
                    span { "{err}" }
                    button {
                        class: "text-red-400 hover:text-red-300",
                        onclick: move |_| error_msg.set(None),
                        Icon { width: 14, height: 14, icon: LdX }
                    }
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

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Name" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Direction" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Source" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Action" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Hits" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*rules.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for rule in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{rule.id}",
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-medium", "{rule.name}" }
                                        td { class: "px-5 py-3 text-sm",
                                            span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20",
                                                match rule.direction { Direction::Ingress => "IN", Direction::Egress => "OUT" }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400", {rule.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono", {rule.src_ip.clone().unwrap_or("*".to_string())} }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono", {rule.dst_ip.clone().unwrap_or("*".to_string())} }
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: match rule.action {
                                                    RuleAction::Allow => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                    RuleAction::Drop => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20",
                                                    _ => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                                },
                                                match rule.action {
                                                    RuleAction::Allow => "ALLOW",
                                                    RuleAction::Drop => "DROP",
                                                    RuleAction::Log => "LOG",
                                                    RuleAction::RateLimit => "RATE LIMIT",
                                                }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm text-cyan-400 font-mono", "{rule.hit_count}" }
                                        td { class: "px-5 py-3 text-sm",
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
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let id = rule.id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/rules/{}", id)).await {
                                                                    Ok(_) => rules.restart(),
                                                                    Err(e) => error_msg.set(Some(e)),
                                                                }
                                                            });
                                                        },
                                                        Icon { width: 13, height: 13, icon: LdTrash2 }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "9", "No firewall rules configured" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "9", "Failed to load rules: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "9", "Loading..." } }
                            },
                        }
                    }
                }
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
            id: 0,
            name: name(),
            priority: priority().parse().unwrap_or(100),
            direction: match direction().as_str() { "Egress" => Direction::Egress, _ => Direction::Ingress },
            enabled: true,
            src_ip: if src_ip().is_empty() { None } else { Some(src_ip()) },
            dst_ip: if dst_ip().is_empty() { None } else { Some(dst_ip()) },
            src_port: None,
            dst_port: if dst_port().is_empty() { None } else { dst_port().parse::<u16>().ok().map(PortRange::single) },
            protocol: match protocol().as_str() { "TCP" => Some(Protocol::TCP), "UDP" => Some(Protocol::UDP), "ICMP" => Some(Protocol::ICMP), _ => None },
            interface: None,
            action: match action().as_str() { "Drop" => RuleAction::Drop, "Log" => RuleAction::Log, _ => RuleAction::Allow },
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
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Create New Rule" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Name" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "Rule name", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Priority" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "number", value: "{priority}",
                        oninput: move |e| priority.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Direction" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        value: "{direction}", onchange: move |e| direction.set(e.value()),
                        option { value: "Ingress", "Ingress" }
                        option { value: "Egress", "Egress" }
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Protocol" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        value: "{protocol}", onchange: move |e| protocol.set(e.value()),
                        option { value: "Any", "Any" }
                        option { value: "TCP", "TCP" }
                        option { value: "UDP", "UDP" }
                        option { value: "ICMP", "ICMP" }
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Source IP" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{src_ip}",
                        oninput: move |e| src_ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Destination IP" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "10.0.0.0/8", value: "{dst_ip}",
                        oninput: move |e| dst_ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Dest Port" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "80", value: "{dst_port}",
                        oninput: move |e| dst_port.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Action" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        value: "{action}", onchange: move |e| action.set(e.value()),
                        option { value: "Allow", "Allow" }
                        option { value: "Drop", "Drop" }
                        option { value: "Log", "Log" }
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Create Rule" }
            }
        }
    }
}
