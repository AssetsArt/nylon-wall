use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;
use super::ui::*;

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct PaginatedLogs {
    entries: Vec<PacketLog>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct PaginatedConntrack {
    total: usize,
    entries: Vec<ConntrackInfo>,
}

#[component]
pub fn Dashboard() -> Element {
    let status = use_resource(|| async { api_client::get::<SystemStatus>("/system/status").await });
    let rules = use_resource(|| async { api_client::get::<Vec<FirewallRule>>("/rules").await });
    let nat_entries = use_resource(|| async { api_client::get::<Vec<NatEntry>>("/nat").await });
    let conns =
        use_resource(|| async { api_client::get::<PaginatedConntrack>("/conntrack").await });
    let recent_logs =
        use_resource(|| async { api_client::get::<PaginatedLogs>("/logs?limit=5").await });
    let dhcp_leases =
        use_resource(|| async { api_client::get::<Vec<DhcpLease>>("/dhcp/leases").await });
    let dhcp_pools =
        use_resource(|| async { api_client::get::<Vec<DhcpPool>>("/dhcp/pools").await });

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
    let lease_count = match &*dhcp_leases.read() {
        Some(Ok(l)) => l
            .iter()
            .filter(|l| l.state == DhcpLeaseState::Active)
            .count(),
        _ => 0,
    };
    let pool_count = match &*dhcp_pools.read() {
        Some(Ok(p)) => p.iter().filter(|p| p.enabled).count(),
        _ => 0,
    };

    let system_value = match &*status.read() {
        Some(Ok(s)) => {
            if s.ebpf_loaded {
                "Online".to_string()
            } else {
                "Limited".to_string()
            }
        }
        Some(Err(_)) => "Offline".to_string(),
        None => "Loading...".to_string(),
    };
    let system_subtitle = match &*status.read() {
        Some(Ok(s)) => Some(format!("v{}", s.version)),
        _ => None,
    };

    rsx! {
        div {
            PageHeader {
                title: "Dashboard".to_string(),
                subtitle: "System overview and statistics".to_string(),
            }

            // Stat cards
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4 mb-8",
                StatCard {
                    color: Color::Emerald,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdActivity, class: "text-emerald-400" } },
                    label: "System".to_string(),
                    value: system_value,
                    subtitle: system_subtitle,
                }
                StatCard {
                    color: Color::Blue,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdShieldCheck, class: "text-blue-400" } },
                    label: "Rules".to_string(),
                    value: format!("{rule_count}"),
                    subtitle: format!("{active_count} active"),
                }
                StatCard {
                    color: Color::Violet,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdArrowLeftRight, class: "text-violet-400" } },
                    label: "NAT".to_string(),
                    value: format!("{nat_count}"),
                    subtitle: "entries".to_string(),
                }
                StatCard {
                    color: Color::Cyan,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdCable, class: "text-cyan-400" } },
                    label: "Connections".to_string(),
                    value: format!("{conn_count}"),
                    subtitle: "active".to_string(),
                }
                StatCard {
                    color: Color::Teal,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdWifi, class: "text-teal-400" } },
                    label: "DHCP".to_string(),
                    value: format!("{lease_count}"),
                    subtitle: format!("{pool_count} pool(s) active"),
                }
            }

            // Recent rules table
            div { class: "mb-6",
                h3 { class: "text-sm font-semibold text-white mb-3", "Recent Rules" }
            }
            div { class: "mb-8",
                DataTable {
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: TH_CLASS, "Name" }
                            th { class: TH_CLASS, "Direction" }
                            th { class: TH_CLASS, "Action" }
                            th { class: TH_CLASS, "Status" }
                        }
                    }
                    tbody {
                        match &*rules.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for rule in list.iter().take(5) {
                                    tr { class: TR_CLASS,
                                        key: "{rule.id}",
                                        td { class: "{TD_CLASS} text-slate-300 font-medium", "{rule.name}" }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: Color::Slate,
                                                label: match rule.direction {
                                                    Direction::Ingress => "IN".to_string(),
                                                    Direction::Egress => "OUT".to_string(),
                                                },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: match rule.action {
                                                    RuleAction::Allow => Color::Emerald,
                                                    RuleAction::Drop => Color::Red,
                                                    _ => Color::Amber,
                                                },
                                                label: match rule.action {
                                                    RuleAction::Allow => "ALLOW".to_string(),
                                                    RuleAction::Drop => "DROP".to_string(),
                                                    RuleAction::Log => "LOG".to_string(),
                                                    RuleAction::RateLimit => "RATE LIMIT".to_string(),
                                                },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if rule.enabled { Color::Emerald } else { Color::Slate },
                                                label: if rule.enabled { "Enabled".to_string() } else { "Disabled".to_string() },
                                            }
                                        }
                                    }
                                }
                            },
                            _ => rsx! {
                                TableEmpty { colspan: 4, message: "No rules configured".to_string() }
                            },
                        }
                    }
                }
            }

            // Recent Logs section
            div { class: "mb-6",
                h3 { class: "text-sm font-semibold text-white mb-3", "Recent Logs" }
            }
            div { class: "mb-8",
                DataTable {
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: TH_CLASS, "Time" }
                            th { class: TH_CLASS, "Source" }
                            th { class: TH_CLASS, "Destination" }
                            th { class: TH_CLASS, "Protocol" }
                            th { class: TH_CLASS, "Action" }
                        }
                    }
                    tbody {
                        match &*recent_logs.read() {
                            Some(Ok(data)) if !data.entries.is_empty() => rsx! {
                                for (i, log) in data.entries.iter().enumerate() {
                                    tr { class: TR_CLASS,
                                        key: "{i}",
                                        td { class: "{TD_CLASS} text-slate-500 font-mono", "{log.timestamp}" }
                                        td { class: "{TD_CLASS} text-slate-300 font-mono",
                                            span { class: "text-slate-300", "{log.src_ip}" }
                                            span { class: "text-slate-600", ":{log.src_port}" }
                                        }
                                        td { class: "{TD_CLASS} text-slate-300 font-mono",
                                            span { class: "text-slate-300", "{log.dst_ip}" }
                                            span { class: "text-slate-600", ":{log.dst_port}" }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: Color::Slate,
                                                label: format!("{}", log.protocol),
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: match log.action.to_uppercase().as_str() {
                                                    "DROP" => Color::Red,
                                                    "LOG" => Color::Amber,
                                                    _ => Color::Emerald,
                                                },
                                                label: format!("{}", log.action),
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                TableEmpty { colspan: 5, message: "No recent logs".to_string() }
                            },
                            Some(Err(e)) => rsx! {
                                TableError { colspan: 5, message: format!("Failed to load logs: {e}") }
                            },
                            None => rsx! {
                                TableLoading { colspan: 5 }
                            },
                        }
                    }
                }
            }

        }
    }
}
