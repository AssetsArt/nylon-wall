use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Dashboard() -> Element {
    let status = use_resource(|| async {
        api_client::get::<SystemStatus>("/system/status").await
    });
    let rules = use_resource(|| async {
        api_client::get::<Vec<FirewallRule>>("/rules").await
    });

    let rule_count = match &*rules.read() {
        Some(Ok(r)) => r.len(),
        _ => 0,
    };
    let active_count = match &*rules.read() {
        Some(Ok(r)) => r.iter().filter(|r| r.enabled).count(),
        _ => 0,
    };

    rsx! {
        div {
            div { class: "mb-6",
                h2 { class: "text-xl font-semibold text-white", "Dashboard" }
                p { class: "text-sm text-slate-400 mt-1", "System overview and statistics" }
            }

            // Stat cards
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8",
                // System Status
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-emerald-500/30 transition-colors",
                    div { class: "flex items-center gap-3 mb-3",
                        div { class: "w-9 h-9 rounded-lg bg-emerald-500/10 flex items-center justify-center",
                            Icon { width: 16, height: 16, icon: LdActivity, class: "text-emerald-400" }
                        }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "System" }
                    }
                    match &*status.read() {
                        Some(Ok(s)) => rsx! {
                            p { class: "text-2xl font-bold text-white mb-1",
                                if s.ebpf_loaded { "Online" } else { "Limited" }
                            }
                            p { class: "text-xs text-slate-500", "v{s.version}" }
                        },
                        Some(Err(_)) => rsx! {
                            p { class: "text-2xl font-bold text-red-400", "Offline" }
                        },
                        None => rsx! {
                            p { class: "text-sm text-slate-600", "Loading..." }
                        },
                    }
                }

                // Rules count
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-blue-500/30 transition-colors",
                    div { class: "flex items-center gap-3 mb-3",
                        div { class: "w-9 h-9 rounded-lg bg-blue-500/10 flex items-center justify-center",
                            Icon { width: 16, height: 16, icon: LdShieldCheck, class: "text-blue-400" }
                        }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Rules" }
                    }
                    p { class: "text-2xl font-bold text-white mb-1", "{rule_count}" }
                    p { class: "text-xs text-slate-500", "{active_count} active" }
                }

                // NAT
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-violet-500/30 transition-colors",
                    div { class: "flex items-center gap-3 mb-3",
                        div { class: "w-9 h-9 rounded-lg bg-violet-500/10 flex items-center justify-center",
                            Icon { width: 16, height: 16, icon: LdArrowLeftRight, class: "text-violet-400" }
                        }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "NAT" }
                    }
                    p { class: "text-2xl font-bold text-white mb-1", "\u{2014}" }
                    p { class: "text-xs text-slate-500", "entries" }
                }

                // Connections
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-cyan-500/30 transition-colors",
                    div { class: "flex items-center gap-3 mb-3",
                        div { class: "w-9 h-9 rounded-lg bg-cyan-500/10 flex items-center justify-center",
                            Icon { width: 16, height: 16, icon: LdNetwork, class: "text-cyan-400" }
                        }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Connections" }
                    }
                    p { class: "text-2xl font-bold text-white mb-1", "\u{2014}" }
                    p { class: "text-xs text-slate-500", "active" }
                }
            }

            // Recent rules table
            div { class: "mb-6",
                h3 { class: "text-sm font-semibold text-white mb-3", "Recent Rules" }
            }
            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Name" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Direction" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Action" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                        }
                    }
                    tbody {
                        match &*rules.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for rule in list.iter().take(5) {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{rule.id}",
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-medium", "{rule.name}" }
                                        td { class: "px-5 py-3 text-sm",
                                            span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20",
                                                match rule.direction {
                                                    Direction::Ingress => "IN",
                                                    Direction::Egress => "OUT",
                                                }
                                            }
                                        }
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
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: if rule.enabled {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                                } else {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20"
                                                },
                                                if rule.enabled { "Enabled" } else { "Disabled" }
                                            }
                                        }
                                    }
                                }
                            },
                            _ => rsx! {
                                tr {
                                    td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "4",
                                        "No rules configured"
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}
