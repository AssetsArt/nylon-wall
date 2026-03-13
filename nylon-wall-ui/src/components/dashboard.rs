use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use std::collections::HashMap;
use crate::api_client;
use crate::models::*;

#[derive(Debug, Clone, serde::Deserialize)]
struct PaginatedLogs {
    entries: Vec<PacketLog>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PaginatedConntrack {
    total: usize,
    entries: Vec<ConntrackInfo>,
}

#[component]
pub fn Dashboard() -> Element {
    let status = use_resource(|| async {
        api_client::get::<SystemStatus>("/system/status").await
    });
    let rules = use_resource(|| async {
        api_client::get::<Vec<FirewallRule>>("/rules").await
    });
    let nat_entries = use_resource(|| async {
        api_client::get::<Vec<NatEntry>>("/nat").await
    });
    let conns = use_resource(|| async {
        api_client::get::<PaginatedConntrack>("/conntrack").await
    });
    let recent_logs = use_resource(|| async {
        api_client::get::<PaginatedLogs>("/logs?limit=5").await
    });
    let blocked_logs = use_resource(|| async {
        api_client::get::<PaginatedLogs>("/logs?action=drop&limit=50").await
    });

    let rule_count = match &*rules.read() {
        Some(Ok(r)) => r.len(),
        _ => 0,
    };
    let active_count = match &*rules.read() {
        Some(Ok(r)) => r.iter().filter(|r| r.enabled).count(),
        _ => 0,
    };
    let nat_count = match &*nat_entries.read() {
        Some(Ok(n)) => n.len(),
        _ => 0,
    };
    let conn_count = match &*conns.read() {
        Some(Ok(c)) => c.total,
        _ => 0,
    };

    // Aggregate top blocked IPs from blocked_logs
    let top_blocked: Vec<(String, usize)> = match &*blocked_logs.read() {
        Some(Ok(data)) => {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for log in data.entries.iter() {
                let action_lower = log.action.to_lowercase();
                if action_lower == "drop" {
                    *counts.entry(log.src_ip.clone()).or_insert(0) += 1;
                }
            }
            let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            sorted.into_iter().take(10).collect()
        }
        _ => Vec::new(),
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
                    p { class: "text-2xl font-bold text-white mb-1", "{nat_count}" }
                    p { class: "text-xs text-slate-500", "entries" }
                }

                // Connections
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-cyan-500/30 transition-colors",
                    div { class: "flex items-center gap-3 mb-3",
                        div { class: "w-9 h-9 rounded-lg bg-cyan-500/10 flex items-center justify-center",
                            Icon { width: 16, height: 16, icon: LdCable, class: "text-cyan-400" }
                        }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Connections" }
                    }
                    p { class: "text-2xl font-bold text-white mb-1", "{conn_count}" }
                    p { class: "text-xs text-slate-500", "active" }
                }
            }

            // Recent rules table
            div { class: "mb-6",
                h3 { class: "text-sm font-semibold text-white mb-3", "Recent Rules" }
            }
            div { class: "rounded-xl border border-slate-800/60 overflow-hidden mb-8",
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

            // Recent Logs section
            div { class: "mb-6",
                h3 { class: "text-sm font-semibold text-white mb-3", "Recent Logs" }
            }
            div { class: "rounded-xl border border-slate-800/60 overflow-hidden mb-8",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Time" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Source" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Action" }
                        }
                    }
                    tbody {
                        match &*recent_logs.read() {
                            Some(Ok(data)) if !data.entries.is_empty() => rsx! {
                                for (i, log) in data.entries.iter().enumerate() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{i}",
                                        td { class: "px-5 py-3 text-sm text-slate-500 font-mono", "{log.timestamp}" }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            span { class: "text-slate-300", "{log.src_ip}" }
                                            span { class: "text-slate-600", ":{log.src_port}" }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            span { class: "text-slate-300", "{log.dst_ip}" }
                                            span { class: "text-slate-600", ":{log.dst_port}" }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20",
                                                "{log.protocol}"
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: match log.action.to_uppercase().as_str() {
                                                    "DROP" => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20",
                                                    "LOG" => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                                    _ => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                },
                                                "{log.action}"
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "5", "No recent logs" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "5", "Failed to load logs: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "5", "Loading..." } }
                            },
                        }
                    }
                }
            }

            // Top Blocked IPs section
            div { class: "mb-6",
                h3 { class: "text-sm font-semibold text-white mb-3", "Top Blocked IPs" }
            }
            if top_blocked.is_empty() {
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-8 text-center",
                    match &*blocked_logs.read() {
                        Some(Ok(_)) => rsx! {
                            p { class: "text-sm text-slate-600", "No blocked traffic detected" }
                        },
                        Some(Err(e)) => rsx! {
                            p { class: "text-sm text-red-400", "Failed to load data: {e}" }
                        },
                        None => rsx! {
                            p { class: "text-sm text-slate-600", "Loading..." }
                        },
                    }
                }
            } else {
                div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3",
                    for (ip, count) in top_blocked.iter() {
                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4 hover:border-red-500/30 transition-colors",
                            key: "{ip}",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-3",
                                    div { class: "w-8 h-8 rounded-lg bg-red-500/10 flex items-center justify-center",
                                        Icon { width: 14, height: 14, icon: LdShieldAlert, class: "text-red-400" }
                                    }
                                    div {
                                        p { class: "text-sm text-slate-300 font-mono font-medium", "{ip}" }
                                        p { class: "text-[11px] text-slate-500", "blocked source" }
                                    }
                                }
                                span { class: "px-2.5 py-1 rounded-full text-xs font-bold bg-red-500/10 text-red-400 border border-red-500/20",
                                    "{count}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
