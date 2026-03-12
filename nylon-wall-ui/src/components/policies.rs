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
    let zones = use_resource(|| async {
        api_client::get::<Vec<Zone>>("/zones").await
    });
    let policies = use_resource(|| async {
        api_client::get::<Vec<NetworkPolicy>>("/policies").await
    });

    rsx! {
        div {
            div { class: "mb-6",
                h2 { class: "text-xl font-semibold text-white", "Network Policies" }
                p { class: "text-sm text-slate-400 mt-1", "Zone definitions and inter-zone traffic policies" }
            }

            // Zones section
            div { class: "mb-8",
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdLayers, class: "text-slate-500" }
                    h3 { class: "text-sm font-semibold text-white", "Zones" }
                }
                match &*zones.read() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4",
                            for (i, zone) in list.iter().enumerate() {
                                {
                                    let color_idx = i % ZONE_COLORS.len();
                                    let (border_cls, _icon_bg, icon_color) = ZONE_COLORS[color_idx];
                                    let card_cls = format!("rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 border-l-4 {border_cls}");
                                    rsx! {
                                        div { class: "{card_cls}", key: "{zone.id}",
                                            div { class: "flex items-center justify-between mb-3",
                                                h4 { class: "text-sm font-semibold text-white", "{zone.name}" }
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
                div { class: "flex items-center gap-2 mb-4",
                    Icon { width: 15, height: 15, icon: LdShieldCheck, class: "text-slate-500" }
                    h3 { class: "text-sm font-semibold text-white", "Inter-Zone Policies" }
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
                                        }
                                    }
                                },
                                Some(Ok(_)) => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "No policies configured" } }
                                },
                                Some(Err(e)) => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "8", "Failed to load policies: {e}" } }
                                },
                                None => rsx! {
                                    tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "Loading..." } }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
