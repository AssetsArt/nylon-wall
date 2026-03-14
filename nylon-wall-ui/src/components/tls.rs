use super::{ConfirmModal, use_change_guard, use_refresh_trigger, notify_change};
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn Tls() -> Element {
    let mut rules = use_resource(|| async {
        api_client::get::<Vec<SniRule>>("/tls/sni/rules").await
    });
    let mut stats = use_resource(|| async {
        api_client::get::<SniStats>("/tls/sni/stats").await
    });
    let mut editing = use_signal(|| None::<(bool, SniRule)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(u32, String)>);
    let mut confirm_toggle = use_signal(|| None::<(u32, String, bool)>);
    let mut confirm_global_toggle = use_signal(|| None::<bool>);
    let mut guard = use_change_guard();

    let refresh = use_refresh_trigger();
    let mut prev_refresh = use_signal(|| refresh());
    use_effect(move || {
        let r = refresh();
        if r != prev_refresh() {
            prev_refresh.set(r);
            rules.restart();
            stats.restart();
        }
    });

    rsx! {
        div {
            PageHeader {
                title: "TLS / SNI Filtering".to_string(),
                subtitle: "Block or log connections based on TLS Server Name Indication".to_string(),
                div { class: "flex items-center gap-2",
                    Btn {
                        color: Color::Blue,
                        label: if editing().is_some() { "Cancel".to_string() } else { "+ New SNI Rule".to_string() },
                        onclick: move |_| {
                            if editing().is_some() {
                                editing.set(None);
                            } else {
                                editing.set(Some((false, SniRule {
                                    id: 0,
                                    domain: String::new(),
                                    action: SniAction::Block,
                                    enabled: true,
                                    hit_count: 0,
                                    category: None,
                                })));
                            }
                        },
                    }
                }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            // Stats cards — clone data out to avoid borrow issues with closures
            {
                let stats_data = stats.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
                rsx! {
                    if let Some(s) = stats_data {
                        div { class: "grid grid-cols-5 gap-4 mb-6",
                            StatCard {
                                label: "Status".to_string(),
                                value: if s.enabled { "Enabled".to_string() } else { "Disabled".to_string() },
                                color: if s.enabled { Color::Emerald } else { Color::Red },
                                icon: rsx! { Icon { width: 16, height: 16, icon: LdShield } },
                            }
                            StatCard {
                                label: "Total Inspected".to_string(),
                                value: s.total_inspected.to_string(),
                                color: Color::Blue,
                                icon: rsx! { Icon { width: 16, height: 16, icon: LdSearch } },
                            }
                            StatCard {
                                label: "Blocked".to_string(),
                                value: s.total_blocked.to_string(),
                                color: Color::Red,
                                icon: rsx! { Icon { width: 16, height: 16, icon: LdShieldX } },
                            }
                            StatCard {
                                label: "Allowed".to_string(),
                                value: s.total_allowed.to_string(),
                                color: Color::Emerald,
                                icon: rsx! { Icon { width: 16, height: 16, icon: LdShieldCheck } },
                            }
                            StatCard {
                                label: "Logged".to_string(),
                                value: s.total_logged.to_string(),
                                color: Color::Amber,
                                icon: rsx! { Icon { width: 16, height: 16, icon: LdEye } },
                            }
                        }

                        // Global toggle button
                        div { class: "mb-6 flex items-center gap-3",
                            {
                                let is_enabled = s.enabled;
                                rsx! {
                                    button {
                                        class: if is_enabled {
                                            "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium bg-red-500/10 text-red-400 ring-1 ring-red-500/20 hover:bg-red-500/20 transition-colors"
                                        } else {
                                            "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20 hover:bg-emerald-500/20 transition-colors"
                                        },
                                        onclick: move |_| {
                                            confirm_global_toggle.set(Some(is_enabled));
                                        },
                                        if is_enabled {
                                            Icon { width: 14, height: 14, icon: LdShieldX }
                                            "Disable SNI Filtering"
                                        } else {
                                            Icon { width: 14, height: 14, icon: LdShieldCheck }
                                            "Enable SNI Filtering"
                                        }
                                    }
                                }
                            }
                            p { class: "text-xs text-slate-500",
                                "When enabled, TLS ClientHello packets are inspected in eBPF for domain-based filtering."
                            }
                        }
                    }
                }
            }

            // Global toggle confirm modal
            if let Some(currently_enabled) = confirm_global_toggle() {
                ConfirmModal {
                    title: if currently_enabled { "Disable SNI Filtering".to_string() } else { "Enable SNI Filtering".to_string() },
                    message: if currently_enabled {
                        "Are you sure you want to disable SNI filtering? All TLS traffic will pass without domain inspection.".to_string()
                    } else {
                        "Enable SNI filtering? TLS ClientHello packets will be inspected in eBPF for domain-based filtering.".to_string()
                    },
                    confirm_label: if currently_enabled { "Disable".to_string() } else { "Enable".to_string() },
                    danger: currently_enabled,
                    on_confirm: move |_| {
                        confirm_global_toggle.set(None);
                        spawn(async move {
                            let new_state = !currently_enabled;
                            match api_client::post::<serde_json::Value, serde_json::Value>(
                                "/tls/sni/toggle",
                                &serde_json::json!({"enabled": new_state}),
                            ).await {
                                Ok(_) => {
                                    stats.restart();
                                    notify_change(&mut guard);
                                }
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_global_toggle.set(None); },
                }
            }

            // Edit/Create form
            if let Some((is_edit, rule)) = editing() {
                SniRuleForm {
                    key: "{rule.id}",
                    is_edit: is_edit,
                    editing: rule,
                    on_saved: move |_| {
                        editing.set(None);
                        rules.restart();
                        stats.restart();
                        notify_change(&mut guard);
                    }
                }
            }

            // Delete confirm modal
            if let Some((del_id, del_domain)) = confirm_delete() {
                ConfirmModal {
                    title: "Delete SNI Rule".to_string(),
                    message: format!("Are you sure you want to delete the SNI rule for \"{}\"? This action cannot be undone.", del_domain),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/tls/sni/rules/{}", del_id)).await {
                                Ok(_) => {
                                    rules.restart();
                                    stats.restart();
                                    notify_change(&mut guard);
                                }
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete.set(None); },
                }
            }

            // Toggle confirm modal
            if let Some((tog_id, tog_domain, tog_enabled)) = confirm_toggle() {
                ConfirmModal {
                    title: if tog_enabled { "Disable SNI Rule".to_string() } else { "Enable SNI Rule".to_string() },
                    message: format!(
                        "Are you sure you want to {} the SNI rule for \"{}\"?",
                        if tog_enabled { "disable" } else { "enable" },
                        tog_domain
                    ),
                    confirm_label: if tog_enabled { "Disable".to_string() } else { "Enable".to_string() },
                    danger: tog_enabled,
                    on_confirm: move |_| {
                        confirm_toggle.set(None);
                        spawn(async move {
                            match api_client::post::<(), serde_json::Value>(
                                &format!("/tls/sni/rules/{}/toggle", tog_id), &()
                            ).await {
                                Ok(_) => {
                                    rules.restart();
                                    stats.restart();
                                    notify_change(&mut guard);
                                }
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_toggle.set(None); },
                }
            }

            // Rules table
            match &*rules.read() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    EmptyState {
                        icon: rsx! { Icon { width: 32, height: 32, icon: LdGlobe } },
                        title: "No SNI Rules".to_string(),
                        subtitle: "Add SNI rules to filter TLS connections by domain name.".to_string(),
                    }
                },
                Some(Ok(list)) => {
                    let mut sorted = list.clone();
                    sorted.sort_by_key(|r| r.id);
                    rsx! {
                        DataTable {
                            thead {
                                tr {
                                    th { class: TH_CLASS, "Domain" }
                                    th { class: TH_CLASS, "Action" }
                                    th { class: TH_CLASS, "Category" }
                                    th { class: TH_CLASS, "Hits" }
                                    th { class: TH_CLASS, "Status" }
                                    th { class: "{TH_CLASS} text-right", "Actions" }
                                }
                            }
                            tbody {
                                for rule in sorted {
                                    {
                                        let rule_c = rule.clone();
                                        let domain_c2 = rule.domain.clone();
                                        let domain_c3 = rule.domain.clone();
                                        rsx! {
                                            tr { class: TR_CLASS,
                                                td { class: TD_CLASS,
                                                    div { class: "flex items-center gap-2",
                                                        Icon { width: 14, height: 14, icon: LdGlobe, class: "text-slate-500" }
                                                        span { class: "font-mono text-slate-200",
                                                            "{rule.domain}"
                                                        }
                                                    }
                                                }
                                                td { class: TD_CLASS,
                                                    Badge {
                                                        color: match rule.action {
                                                            SniAction::Block => Color::Red,
                                                            SniAction::Allow => Color::Emerald,
                                                            SniAction::Log => Color::Amber,
                                                        },
                                                        label: match rule.action {
                                                            SniAction::Block => "Block".to_string(),
                                                            SniAction::Allow => "Allow".to_string(),
                                                            SniAction::Log => "Log".to_string(),
                                                        },
                                                    }
                                                }
                                                td { class: "{TD_CLASS} text-slate-400",
                                                    {rule.category.as_deref().unwrap_or("-")}
                                                }
                                                td { class: "{TD_CLASS} text-slate-300 font-mono",
                                                    "{rule.hit_count}"
                                                }
                                                td { class: TD_CLASS,
                                                    button {
                                                        class: "cursor-pointer",
                                                        onclick: move |_| {
                                                            confirm_toggle.set(Some((rule.id, domain_c2.clone(), rule.enabled)));
                                                        },
                                                        Badge {
                                                            color: if rule.enabled { Color::Emerald } else { Color::Red },
                                                            label: if rule.enabled { "Enabled".to_string() } else { "Disabled".to_string() },
                                                        }
                                                    }
                                                }
                                                td { class: TD_CLASS,
                                                    div { class: "flex items-center justify-end gap-1",
                                                        EditBtn {
                                                            onclick: move |_| {
                                                                editing.set(Some((true, rule_c.clone())));
                                                            },
                                                        }
                                                        DeleteBtn {
                                                            onclick: move |_| {
                                                                confirm_delete.set(Some((rule.id, domain_c3.clone())));
                                                            },
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
                Some(Err(e)) => rsx! {
                    TableError { message: e.clone() }
                },
                None => rsx! {
                    TableLoading {}
                },
            }
        }
    }
}

// === SNI Rule Form ===

#[component]
fn SniRuleForm(
    is_edit: bool,
    editing: SniRule,
    on_saved: EventHandler<()>,
) -> Element {
    let mut domain = use_signal(|| editing.domain.clone());
    let mut action = use_signal(|| match editing.action {
        SniAction::Block => "Block".to_string(),
        SniAction::Allow => "Allow".to_string(),
        SniAction::Log => "Log".to_string(),
    });
    let mut category = use_signal(|| editing.category.clone().unwrap_or_default());
    let mut enabled = use_signal(|| editing.enabled);
    let mut error_msg = use_signal(|| None::<String>);

    let edit_id = editing.id;
    let edit_hit_count = editing.hit_count;

    rsx! {
        FormCard {
            p { class: "text-lg font-semibold text-white mb-4",
                if is_edit { "Edit SNI Rule" } else { "New SNI Rule" }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            div { class: "grid grid-cols-4 gap-4",
                FormField { label: "Domain".to_string(),
                    input {
                        class: "w-full bg-slate-800/50 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-blue-500",
                        r#type: "text",
                        placeholder: "e.g. facebook.com or *.tiktok.com",
                        value: "{domain}",
                        oninput: move |e| domain.set(e.value()),
                    }
                }

                FormField { label: "Action".to_string(),
                    select {
                        class: "w-full bg-slate-800/50 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-blue-500",
                        value: "{action}",
                        onchange: move |e| action.set(e.value()),
                        option { value: "Block", "Block" }
                        option { value: "Allow", "Allow" }
                        option { value: "Log", "Log Only" }
                    }
                }

                FormField { label: "Category".to_string(),
                    input {
                        class: "w-full bg-slate-800/50 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-blue-500",
                        r#type: "text",
                        placeholder: "e.g. social, ads, malware",
                        value: "{category}",
                        oninput: move |e| category.set(e.value()),
                    }
                }

                FormField { label: "Enabled".to_string(),
                    select {
                        class: "w-full bg-slate-800/50 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-1 focus:ring-blue-500",
                        value: if enabled() { "true" } else { "false" },
                        onchange: move |e| enabled.set(e.value() == "true"),
                        option { value: "true", "Yes" }
                        option { value: "false", "No" }
                    }
                }
            }

            div { class: "flex justify-end mt-4",
                SubmitBtn {
                    color: Color::Blue,
                    label: if is_edit { "Update Rule".to_string() } else { "Create Rule".to_string() },
                    onclick: move |_| {
                        let domain_val = domain().trim().to_string();
                        if domain_val.is_empty() {
                            error_msg.set(Some("Domain is required".to_string()));
                            return;
                        }

                        let sni_action = match action().as_str() {
                            "Allow" => SniAction::Allow,
                            "Log" => SniAction::Log,
                            _ => SniAction::Block,
                        };

                        let cat = {
                            let c = category().trim().to_string();
                            if c.is_empty() { None } else { Some(c) }
                        };

                        let rule = SniRule {
                            id: edit_id,
                            domain: domain_val,
                            action: sni_action,
                            enabled: enabled(),
                            hit_count: edit_hit_count,
                            category: cat,
                        };

                        spawn(async move {
                            let result = if is_edit {
                                api_client::put::<SniRule, SniRule>(
                                    &format!("/tls/sni/rules/{}", edit_id), &rule
                                ).await
                            } else {
                                api_client::post::<SniRule, SniRule>("/tls/sni/rules", &rule).await
                            };

                            match result {
                                Ok(_) => on_saved.call(()),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                }
            }
        }
    }
}
