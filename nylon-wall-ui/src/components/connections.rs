use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;
use nylon_wall_common::conntrack::ConnState;

const PAGE_SIZE: usize = 25;

#[derive(Debug, Clone, serde::Deserialize)]
struct PaginatedConntrack {
    total: usize,
    offset: usize,
    limit: usize,
    entries: Vec<ConntrackInfo>,
}

#[component]
pub fn Connections() -> Element {
    let mut current_page = use_signal(|| 0usize);
    let mut filter_state = use_signal(|| "all".to_string());
    let mut filter_protocol = use_signal(|| "all".to_string());

    let page = current_page();
    let state_filter = filter_state();
    let protocol_filter = filter_protocol();

    let mut conns = use_resource(move || {
        let offset = page * PAGE_SIZE;
        let state_val = state_filter.clone();
        let proto_val = protocol_filter.clone();
        async move {
            let mut url = format!("/conntrack?limit={}&offset={}", PAGE_SIZE, offset);
            if state_val != "all" {
                url.push_str(&format!("&state={}", state_val));
            }
            if proto_val != "all" {
                url.push_str(&format!("&protocol={}", proto_val));
            }
            api_client::get::<PaginatedConntrack>(&url).await
        }
    });

    let (entries, total) = match &*conns.read() {
        Some(Ok(data)) => (data.entries.clone(), data.total),
        _ => (vec![], 0),
    };

    let total_pages = if total == 0 { 1 } else { (total + PAGE_SIZE - 1) / PAGE_SIZE };
    let established = entries.iter().filter(|c| c.state == ConnState::Established).count();
    let new_count = entries.iter().filter(|c| c.state == ConnState::New).count();

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "Connections" }
                    p { class: "text-sm text-slate-400 mt-1", "Active connection tracking table" }
                }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors flex items-center gap-1.5",
                    onclick: move |_| { conns.restart(); },
                    Icon { width: 13, height: 13, icon: LdRefreshCw }
                    "Refresh"
                }
            }

            // Stats bar
            div { class: "grid grid-cols-3 gap-4 mb-6",
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4",
                    div { class: "flex items-center gap-2 mb-1",
                        div { class: "w-2 h-2 rounded-full bg-cyan-400" }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Total" }
                    }
                    p { class: "text-xl font-bold text-white", "{total}" }
                }
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4",
                    div { class: "flex items-center gap-2 mb-1",
                        div { class: "w-2 h-2 rounded-full bg-emerald-400" }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Established" }
                    }
                    p { class: "text-xl font-bold text-white", "{established}" }
                }
                div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4",
                    div { class: "flex items-center gap-2 mb-1",
                        div { class: "w-2 h-2 rounded-full bg-amber-400" }
                        span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "New" }
                    }
                    p { class: "text-xl font-bold text-white", "{new_count}" }
                }
            }

            // Filters
            div { class: "flex items-center gap-4 mb-4",
                div { class: "flex items-center gap-2",
                    span { class: "text-xs font-medium text-slate-500", "State:" }
                    select {
                        class: "bg-slate-900 border border-slate-700/60 rounded-lg px-2 py-1 text-xs text-slate-300 outline-none focus:border-blue-500/60",
                        value: "{filter_state}",
                        onchange: move |e| {
                            filter_state.set(e.value());
                            current_page.set(0);
                        },
                        option { value: "all", "All" }
                        option { value: "new", "New" }
                        option { value: "established", "Established" }
                        option { value: "related", "Related" }
                        option { value: "invalid", "Invalid" }
                    }
                }
                div { class: "flex items-center gap-2",
                    span { class: "text-xs font-medium text-slate-500", "Protocol:" }
                    select {
                        class: "bg-slate-900 border border-slate-700/60 rounded-lg px-2 py-1 text-xs text-slate-300 outline-none focus:border-blue-500/60",
                        value: "{filter_protocol}",
                        onchange: move |e| {
                            filter_protocol.set(e.value());
                            current_page.set(0);
                        },
                        option { value: "all", "All" }
                        option { value: "tcp", "TCP" }
                        option { value: "udp", "UDP" }
                        option { value: "icmp", "ICMP" }
                    }
                }
                span { class: "text-xs text-slate-600 ml-auto",
                    {format!("Showing {}\u{2013}{} of {}", page * PAGE_SIZE + 1, (page * PAGE_SIZE + entries.len()).min(total), total)}
                }
            }

            // Connections table
            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Source" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "State" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Packets" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Bytes" }
                        }
                    }
                    tbody {
                        match &*conns.read() {
                            Some(Ok(data)) if !data.entries.is_empty() => rsx! {
                                for (i, conn) in data.entries.iter().enumerate() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{i}",
                                        td { class: "px-5 py-3 text-sm",
                                            span { class: "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20",
                                                "{conn.protocol}"
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            span { class: "text-slate-300", "{conn.src_ip}" }
                                            span { class: "text-slate-600", ":{conn.src_port}" }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            span { class: "text-slate-300", "{conn.dst_ip}" }
                                            span { class: "text-slate-600", ":{conn.dst_port}" }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: match conn.state {
                                                    ConnState::Established => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                    ConnState::New => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20",
                                                    ConnState::Related => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20",
                                                    ConnState::Invalid => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20",
                                                },
                                                "{conn.state}"
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono",
                                            span { class: "text-emerald-400", "{conn.packets_in}" }
                                            span { class: "text-slate-600", " / " }
                                            span { class: "text-blue-400", "{conn.packets_out}" }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono",
                                            {format_bytes(conn.bytes_in)}
                                            span { class: "text-slate-600", " / " }
                                            {format_bytes(conn.bytes_out)}
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "No active connections" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "6", "Failed to load: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "Loading..." } }
                            },
                        }
                    }
                }
            }

            // Pagination controls
            if total_pages > 1 {
                div { class: "flex items-center justify-between mt-4",
                    div { class: "flex items-center gap-1",
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors disabled:opacity-30",
                            class: "bg-slate-800/50 text-slate-400 border-slate-700/40 hover:bg-slate-700/50",
                            disabled: page == 0,
                            onclick: move |_| { current_page.set(0); },
                            Icon { width: 12, height: 12, icon: LdChevronsLeft }
                        }
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors disabled:opacity-30",
                            class: "bg-slate-800/50 text-slate-400 border-slate-700/40 hover:bg-slate-700/50",
                            disabled: page == 0,
                            onclick: move |_| { current_page.set(page.saturating_sub(1)); },
                            Icon { width: 12, height: 12, icon: LdChevronLeft }
                        }

                        // Page numbers
                        {
                            let start = page.saturating_sub(2);
                            let end = (start + 5).min(total_pages);
                            rsx! {
                                for p in start..end {
                                    button {
                                        key: "{p}",
                                        class: "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors",
                                        class: if p == page {
                                            "bg-blue-500/20 text-blue-400 border-blue-500/30"
                                        } else {
                                            "bg-slate-800/50 text-slate-400 border-slate-700/40 hover:bg-slate-700/50"
                                        },
                                        onclick: move |_| { current_page.set(p); },
                                        "{p + 1}"
                                    }
                                }
                            }
                        }

                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors disabled:opacity-30",
                            class: "bg-slate-800/50 text-slate-400 border-slate-700/40 hover:bg-slate-700/50",
                            disabled: page + 1 >= total_pages,
                            onclick: move |_| { current_page.set(page + 1); },
                            Icon { width: 12, height: 12, icon: LdChevronRight }
                        }
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors disabled:opacity-30",
                            class: "bg-slate-800/50 text-slate-400 border-slate-700/40 hover:bg-slate-700/50",
                            disabled: page + 1 >= total_pages,
                            onclick: move |_| { current_page.set(total_pages - 1); },
                            Icon { width: 12, height: 12, icon: LdChevronsRight }
                        }
                    }

                    span { class: "text-xs text-slate-600",
                        "Page {page + 1} of {total_pages}"
                    }
                }
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
