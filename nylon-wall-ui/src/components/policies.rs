use super::ConfirmModal;
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

const ZONE_COLORS: [(&str, &str, &str); 4] = [
    ("border-l-blue-500", "bg-blue-500/10", "text-blue-400"),
    ("border-l-violet-500", "bg-violet-500/10", "text-violet-400"),
    (
        "border-l-emerald-500",
        "bg-emerald-500/10",
        "text-emerald-400",
    ),
    ("border-l-amber-500", "bg-amber-500/10", "text-amber-400"),
];

const DAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn format_schedule(schedule: &Schedule) -> String {
    let days: Vec<&str> = schedule
        .days
        .iter()
        .filter_map(|&d| DAY_LABELS.get(d as usize).copied())
        .collect();
    format!(
        "{} {}-{}",
        days.join(","),
        schedule.start_time,
        schedule.end_time
    )
}

fn action_color(action: &RuleAction) -> Color {
    match action {
        RuleAction::Allow => Color::Emerald,
        RuleAction::Drop => Color::Red,
        _ => Color::Amber,
    }
}

fn action_label(action: &RuleAction) -> &'static str {
    match action {
        RuleAction::Allow => "ALLOW",
        RuleAction::Drop => "DROP",
        RuleAction::Log => "LOG",
        RuleAction::RateLimit => "RATE LIMIT",
    }
}

#[component]
pub fn Policies() -> Element {
    let mut zones = use_resource(|| async { api_client::get::<Vec<Zone>>("/zones").await });
    let mut policies =
        use_resource(|| async { api_client::get::<Vec<NetworkPolicy>>("/policies").await });
    let mut show_zone_form = use_signal(|| false);
    let mut show_policy_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete_zone = use_signal(|| None::<(u32, String)>);
    let mut confirm_delete_policy = use_signal(|| None::<(u32, String)>);

    rsx! {
        div {
            PageHeader {
                title: "Network Policies".to_string(),
                subtitle: "Zone definitions and inter-zone traffic policies".to_string(),
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            if let Some((del_id, del_name)) = confirm_delete_zone() {
                ConfirmModal {
                    title: "Delete Zone".to_string(),
                    message: format!("Are you sure you want to delete zone \"{}\"? This action cannot be undone.", del_name),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete_zone.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/zones/{}", del_id)).await {
                                Ok(_) => zones.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete_zone.set(None); },
                }
            }

            if let Some((del_id, del_name)) = confirm_delete_policy() {
                ConfirmModal {
                    title: "Delete Policy".to_string(),
                    message: format!("Are you sure you want to delete policy \"{}\"? This action cannot be undone.", del_name),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete_policy.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/policies/{}", del_id)).await {
                                Ok(_) => policies.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete_policy.set(None); },
                }
            }

            // Zones section
            div { class: "mb-8",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdLayers, class: "text-slate-500" } },
                    title: "Zones".to_string(),
                    Btn {
                        color: Color::Blue,
                        label: if show_zone_form() { "Cancel".to_string() } else { "+ Add Zone".to_string() },
                        onclick: move |_| show_zone_form.set(!show_zone_form()),
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
                                                    Badge {
                                                        color: action_color(&zone.default_policy),
                                                        label: action_label(&zone.default_policy).to_string(),
                                                    }
                                                    DeleteBtn {
                                                        onclick: {
                                                            let zname = zone.name.clone();
                                                            move |_| {
                                                                confirm_delete_zone.set(Some((zone_id, zname.clone())));
                                                            }
                                                        },
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
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdShieldCheck, class: "text-slate-500" } },
                    title: "Inter-Zone Policies".to_string(),
                    Btn {
                        color: Color::Blue,
                        label: if show_policy_form() { "Cancel".to_string() } else { "+ Add Policy".to_string() },
                        onclick: move |_| show_policy_form.set(!show_policy_form()),
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

                DataTable {
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: TH_CLASS, "Name" }
                            th { class: TH_CLASS, "From" }
                            th { class: TH_CLASS, "To" }
                            th { class: TH_CLASS, "Protocol" }
                            th { class: TH_CLASS, "Action" }
                            th { class: TH_CLASS, "Schedule" }
                            th { class: TH_CLASS, "Priority" }
                            th { class: TH_CLASS, "Log" }
                            th { class: TH_CLASS, "Status" }
                            th { class: TH_CLASS, "" }
                        }
                    }
                    tbody {
                        match &*policies.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for policy in list.iter() {
                                    tr { class: TR_CLASS,
                                        key: "{policy.id}",
                                        td { class: "{TD_CLASS} text-slate-300 font-medium", "{policy.name}" }
                                        td { class: TD_CLASS,
                                            Badge { color: Color::Blue, label: policy.from_zone.clone() }
                                        }
                                        td { class: TD_CLASS,
                                            Badge { color: Color::Violet, label: policy.to_zone.clone() }
                                        }
                                        td { class: "{TD_CLASS} text-slate-400", {policy.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: action_color(&policy.action),
                                                label: action_label(&policy.action).to_string(),
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            match &policy.schedule {
                                                Some(sched) => rsx! {
                                                    span { class: Color::Cyan.badge_class(),
                                                        Icon { width: 10, height: 10, icon: LdClock, class: "inline mr-1" }
                                                        {format_schedule(sched)}
                                                    }
                                                },
                                                None => rsx! {
                                                    span { class: "text-slate-600 text-xs", "Always" }
                                                },
                                            }
                                        }
                                        td { class: "{TD_CLASS} text-cyan-400 font-mono", "{policy.priority}" }
                                        td { class: TD_CLASS,
                                            if policy.log {
                                                Badge { color: Color::Amber, label: "Yes".to_string() }
                                            } else {
                                                span { class: "text-slate-600 text-xs", "No" }
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if policy.enabled { Color::Emerald } else { Color::Slate },
                                                label: if policy.enabled { "Active".to_string() } else { "Inactive".to_string() },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            {
                                                let policy_id = policy.id;
                                                let pname = policy.name.clone();
                                                rsx! {
                                                    DeleteBtn {
                                                        onclick: move |_| {
                                                            confirm_delete_policy.set(Some((policy_id, pname.clone())));
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                TableEmpty { colspan: 10, message: "No policies configured".to_string() }
                            },
                            Some(Err(e)) => rsx! {
                                TableError { colspan: 10, message: format!("Failed to load policies: {e}") }
                            },
                            None => rsx! {
                                TableLoading { colspan: 10 }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ZoneForm(on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(String::new);
    let mut interfaces = use_signal(String::new);
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
        FormCard {
            h3 { class: "text-sm font-semibold text-white mb-4", "Create New Zone" }
            if let Some(err) = error() {
                ErrorAlert { message: err, on_dismiss: move |_| error.set(None) }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-4",
                FormField { label: "Name".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "Zone name", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                FormField { label: "Interfaces".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "eth0, eth1", value: "{interfaces}",
                        oninput: move |e| interfaces.set(e.value()),
                    }
                }
                FormField { label: "Default Policy".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{default_policy}", onchange: move |e| default_policy.set(e.value()),
                        option { value: "Allow", "Allow" }
                        option { value: "Drop", "Drop" }
                    }
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() { "Creating...".to_string() } else { "Create Zone".to_string() },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}

#[component]
fn PolicyForm(on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(String::new);
    let mut from_zone = use_signal(String::new);
    let mut to_zone = use_signal(String::new);
    let mut protocol = use_signal(|| "Any".to_string());
    let mut dst_port = use_signal(String::new);
    let mut action = use_signal(|| "Allow".to_string());
    let mut priority = use_signal(|| "100".to_string());
    let mut log = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    // Schedule fields
    let mut enable_schedule = use_signal(|| false);
    let mut sched_days = use_signal(|| vec![false; 7]);
    let mut sched_start = use_signal(|| "08:00".to_string());
    let mut sched_end = use_signal(|| "18:00".to_string());

    let on_submit = move |_| {
        submitting.set(true);

        let schedule = if enable_schedule() {
            let days: Vec<u8> = sched_days()
                .iter()
                .enumerate()
                .filter_map(|(i, &checked)| if checked { Some(i as u8) } else { None })
                .collect();
            if days.is_empty() {
                error.set(Some(
                    "Please select at least one day for the schedule".to_string(),
                ));
                submitting.set(false);
                return;
            }
            Some(Schedule {
                days,
                start_time: sched_start(),
                end_time: sched_end(),
            })
        } else {
            None
        };

        let policy = NetworkPolicy {
            id: 0,
            name: name(),
            enabled: true,
            from_zone: from_zone(),
            to_zone: to_zone(),
            src_ip: None,
            dst_ip: None,
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
            schedule,
            action: match action().as_str() {
                "Drop" => RuleAction::Drop,
                "Log" => RuleAction::Log,
                _ => RuleAction::Allow,
            },
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
        FormCard {
            h3 { class: "text-sm font-semibold text-white mb-4", "Create New Policy" }
            if let Some(err) = error() {
                ErrorAlert { message: err, on_dismiss: move |_| error.set(None) }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                FormField { label: "Name".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "Policy name", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                FormField { label: "From Zone".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "e.g. lan", value: "{from_zone}",
                        oninput: move |e| from_zone.set(e.value()),
                    }
                }
                FormField { label: "To Zone".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "e.g. wan", value: "{to_zone}",
                        oninput: move |e| to_zone.set(e.value()),
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
                FormField { label: "Dest Port".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "80 (optional)", value: "{dst_port}",
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
                FormField { label: "Priority".to_string(),
                    input {
                        class: INPUT_CLASS,
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

            // Schedule section
            div { class: "mb-4",
                label { class: "flex items-center gap-2 cursor-pointer mb-3",
                    input {
                        r#type: "checkbox",
                        class: "w-4 h-4 rounded border-slate-700/60 bg-slate-900 text-blue-500 focus:ring-blue-500/60",
                        checked: "{enable_schedule}",
                        onchange: move |e| enable_schedule.set(e.checked()),
                    }
                    Icon { width: 14, height: 14, icon: LdClock, class: "text-slate-400" }
                    span { class: "text-sm text-slate-400", "Time-based schedule" }
                }

                if enable_schedule() {
                    div { class: "rounded-lg border border-slate-700/40 bg-slate-900/30 p-4",
                        div { class: "mb-3",
                            label { class: "text-xs font-medium text-slate-400 mb-2 block", "Active Days" }
                            div { class: "flex gap-2",
                                for (i, day_label) in DAY_LABELS.iter().enumerate() {
                                    {
                                        let is_checked = sched_days().get(i).copied().unwrap_or(false);
                                        let btn_cls = if is_checked {
                                            "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/20 text-blue-400 border border-blue-500/30 cursor-pointer transition-colors"
                                        } else {
                                            "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-500 border border-slate-700/40 cursor-pointer hover:border-slate-600 transition-colors"
                                        };
                                        rsx! {
                                            button {
                                                class: "{btn_cls}",
                                                onclick: move |_| {
                                                    let mut days = sched_days();
                                                    if let Some(val) = days.get_mut(i) {
                                                        *val = !*val;
                                                    }
                                                    sched_days.set(days);
                                                },
                                                "{day_label}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "grid grid-cols-2 gap-4",
                            FormField { label: "Start Time".to_string(),
                                input {
                                    class: INPUT_CLASS,
                                    r#type: "time", value: "{sched_start}",
                                    oninput: move |e| sched_start.set(e.value()),
                                }
                            }
                            FormField { label: "End Time".to_string(),
                                input {
                                    class: INPUT_CLASS,
                                    r#type: "time", value: "{sched_end}",
                                    oninput: move |e| sched_end.set(e.value()),
                                }
                            }
                        }
                    }
                }
            }

            SubmitBtn {
                color: Color::Blue,
                label: if submitting() { "Creating...".to_string() } else { "Create Policy".to_string() },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}
