use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

const ZONE_COLORS: [(&str, &str, &str); 4] = [
    ("border-l-blue-500", "bg-blue-500/10", "text-blue-400"),
    ("border-l-violet-500", "bg-violet-500/10", "text-violet-400"),
    ("border-l-emerald-500", "bg-emerald-500/10", "text-emerald-400"),
    ("border-l-amber-500", "bg-amber-500/10", "text-amber-400"),
];

#[component]
pub fn Policies() -> Element {
    let mut zones = use_resource(|| async {
        api_client::get::<Vec<Zone>>("/zones").await
    });
    let mut policies = use_resource(|| async {
        api_client::get::<Vec<NetworkPolicy>>("/policies").await
    });
    let mut show_zone_form = use_signal(|| false);
    let mut show_policy_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "mb-6",
                h2 { class: "text-xl font-semibold text-white", "Network Policies" }
                p { class: "text-sm text-slate-400 mt-1", "Zone definitions and inter-zone traffic policies" }
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

            // Zones section
            div { class: "mb-8",
                div { class: "flex items-center justify-between mb-4",
                    div { class: "flex items-center gap-2",
                        Icon { width: 15, height: 15, icon: LdLayers, class: "text-slate-500" }
                        h3 { class: "text-sm font-semibold text-white", "Zones" }
                    }
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                        onclick: move |_| show_zone_form.set(!show_zone_form()),
                        if show_zone_form() { "Cancel" } else { "+ Add Zone" }
                    }
                }

                if show_zone_form() {
                    ZoneForm {
                        on_saved: move |_| {
                            show_zone_form.set(false);
                            zones.restart();
                        }
                    }
                }

                match &*zones.read() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4",
                            for (i, zone) in list.iter().enumerate() {
                                {
                                    let color_idx = i % ZONE_COLORS.len();
                                    let (border_cls, _icon_bg, icon_color) = ZONE_COLORS[color_idx];
                                    let card_cls = format!("rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 border-l-4 {border_cls}");
                                    let zone_id = zone.id;
                                    rsx! {
                                        div { class: "{card_cls}", key: "{zone.id}",
                                            div { class: "flex items-center justify-between mb-3",
                                                h4 { class: "text-sm font-semibold text-white", "{zone.name}" }
                                                div { class: "flex items-center gap-2",
                                                    span {
                                                        class: match zone.default_policy {
                                                            RuleAction::Allow => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                            RuleAction::Drop => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20",
                                                            _ => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                                        },
                                                        match zone.default_policy {
                                                            RuleAction::Allow => "ALLOW",
                                                            RuleAction::Drop => "DROP",
                                                            RuleAction::Log => "LOG",
                                                            RuleAction::RateLimit => "RATE LIMIT",
                                                        }
                                                    }
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/zones/{}", zone_id)).await {
                                                                    Ok(_) => zones.restart(),
                                                                    Err(e) => error_msg.set(Some(e)),
                                                                }
                                                            });
                                                        },
                                                        Icon { width: 13, height: 13, icon: LdTrash2 }
                                                    }
                                                }
                                            }
                                            div { class: "flex items-center gap-1.5 text-xs text-slate-500",
                                                Icon { width: 12, height: 12, icon: LdNetwork, class: "{icon_color}" }
                                                span {
                                                    {if zone.interfaces.is_empty() {
                                                        "No interfaces".to_string()
                                                    } else {
                                                        zone.interfaces.join(", ")
                                                    }}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-8 text-center",
                            p { class: "text-sm text-slate-600", "No zones configured" }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "rounded-xl border border-red-500/20 bg-red-500/5 p-4",
                            p { class: "text-sm text-red-400", "Failed to load zones: {e}" }
                        }
                    },
                    None => rsx! {
                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-8 text-center",
                            p { class: "text-sm text-slate-600", "Loading..." }
                        }
                    },
                }
            }

            // Policies section
            div {
                div { class: "flex items-center justify-between mb-4",
                    div { class: "flex items-center gap-2",
                        Icon { width: 15, height: 15, icon: LdShieldCheck, class: "text-slate-500" }
                        h3 { class: "text-sm font-semibold text-white", "Inter-Zone Policies" }
                    }
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                        onclick: move |_| show_policy_form.set(!show_policy_form()),
                        if show_policy_form() { "Cancel" } else { "+ Add Policy" }
                    }
                }

                if show_policy_form() {
                    PolicyForm {
                        on_saved: move |_| {
                            show_policy_form.set(false);
                            policies.restart();
                        }
                    }
                }

                div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                    table { class: "w-full text-left",
                        thead { class: "bg-slate-900/80",
                            tr {
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Name" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "From" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "To" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Action" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Priority" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Log" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                                th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                            }
                        }
                        tbody {
                            match &*policies.read() {
                                Some(Ok(list)) if !list.is_empty() => rsx! {
                                    for policy in list.iter() {
                                        tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                            key: "{policy.id}",
                                            td { class: "px-5 py-3 text-sm text-slate-300 font-medium", "{policy.name}" }
                                            td { class: "px-5 py-3 text-sm",
                                                span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20",
                                                    "{policy.from_zone}"
                                                }
                                            }
                                            td { class: "px-5 py-3 text-sm",
                                                span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20",
                                                    "{policy.to_zone}"
                                                }
                                            }
                                            td { class: "px-5 py-3 text-sm text-slate-400", {policy.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                            td { class: "px-5 py-3 text-sm",
                                                span {
                                                    class: match policy.action {
                                                        RuleAction::Allow => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                        RuleAction::Drop => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20",
                                                        _ => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                                    },
                                                    match policy.action {
                                                        RuleAction::Allow => "ALLOW",
                                                        RuleAction::Drop => "DROP",
                                                        RuleAction::Log => "LOG",
                                                        RuleAction::RateLimit => "RATE LIMIT",
                                                    }
                                                }
                                            }
                                            td { class: "px-5 py-3 text-sm text-cyan-400 font-mono", "{policy.priority}" }
                                            td { class: "px-5 py-3 text-sm",
                                                if policy.log {
                                                    span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20", "Yes" }
                                                } else {
                                                    span { class: "text-slate-600 text-xs", "No" }
                                                }
                                            }
                                            td { class: "px-5 py-3 text-sm",
                                                span {
                                                    class: if policy.enabled {
                                                        "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                                    } else {
                                                        "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20"
                                                    },
                                                    if policy.enabled { "Active" } else { "Inactive" }
                                                }
                                            }
                                            td { class: "px-5 py-3 text-sm",
                                                {
                                                    let policy_id = policy.id;
                                                    rsx! {
                                                        button {
                                                            class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                            onclick: move |_| {
                                                                spawn(async move {
                                                                    match api_client::delete(&format!("/policies/{}", policy_id)).await {
                                                                        Ok(_) => policies.restart(),
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
                                    tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "9", "No policies configured" } }
                                },
                                Some(Err(e)) => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "9", "Failed to load policies: {e}" } }
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
}

#[component]
fn ZoneForm(on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(|| String::new());
    let mut interfaces = use_signal(|| String::new());
    let mut default_policy = use_signal(|| "Allow".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let zone = Zone {
            id: 0,
            name: name(),
            interfaces: interfaces()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            default_policy: match default_policy().as_str() {
                "Drop" => RuleAction::Drop,
                _ => RuleAction::Allow,
            },
        };
        spawn(async move {
            match api_client::post::<Zone, Zone>("/zones", &zone).await {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Create New Zone" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Name" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "Zone name", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Interfaces" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "eth0, eth1", value: "{interfaces}",
                        oninput: move |e| interfaces.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Default Policy" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        value: "{default_policy}", onchange: move |e| default_policy.set(e.value()),
                        option { value: "Allow", "Allow" }
                        option { value: "Drop", "Drop" }
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Create Zone" }
            }
        }
    }
}

#[component]
fn PolicyForm(on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(|| String::new());
    let mut from_zone = use_signal(|| String::new());
    let mut to_zone = use_signal(|| String::new());
    let mut protocol = use_signal(|| "Any".to_string());
    let mut dst_port = use_signal(|| String::new());
    let mut action = use_signal(|| "Allow".to_string());
    let mut priority = use_signal(|| "100".to_string());
    let mut log = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let policy = NetworkPolicy {
            id: 0,
            name: name(),
            enabled: true,
            from_zone: from_zone(),
            to_zone: to_zone(),
            src_ip: None,
            dst_ip: None,
            dst_port: if dst_port().is_empty() { None } else { dst_port().parse::<u16>().ok().map(PortRange::single) },
            protocol: match protocol().as_str() { "TCP" => Some(Protocol::TCP), "UDP" => Some(Protocol::UDP), "ICMP" => Some(Protocol::ICMP), _ => None },
            schedule: None,
            action: match action().as_str() { "Drop" => RuleAction::Drop, "Log" => RuleAction::Log, _ => RuleAction::Allow },
            log: log(),
            priority: priority().parse().unwrap_or(100),
        };
        spawn(async move {
            match api_client::post::<NetworkPolicy, NetworkPolicy>("/policies", &policy).await {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Create New Policy" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Name" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "Policy name", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "From Zone" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "e.g. lan", value: "{from_zone}",
                        oninput: move |e| from_zone.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "To Zone" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "e.g. wan", value: "{to_zone}",
                        oninput: move |e| to_zone.set(e.value()),
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
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Dest Port" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "80 (optional)", value: "{dst_port}",
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
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Priority" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "number", value: "{priority}",
                        oninput: move |e| priority.set(e.value()),
                    }
                }
                div { class: "flex items-end pb-1",
                    label { class: "flex items-center gap-2 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            class: "w-4 h-4 rounded border-slate-700/60 bg-slate-900 text-blue-500 focus:ring-blue-500/60",
                            checked: "{log}",
                            onchange: move |e| log.set(e.checked()),
                        }
                        span { class: "text-sm text-slate-400", "Enable Logging" }
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Create Policy" }
            }
        }
    }
}
