use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use nylon_wall_common::conntrack::ConnState;
use super::ui::*;

const PAGE_SIZE: usize = 25;

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
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

    let mut conns = use_resource(move || {
        let page = current_page();
        let offset = page * PAGE_SIZE;
        let state_val = filter_state();
        let proto_val = filter_protocol();
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

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(PAGE_SIZE)
    };
    let established = entries
        .iter()
        .filter(|c| c.state == ConnState::Established)
        .count();
    let new_count = entries.iter().filter(|c| c.state == ConnState::New).count();

    rsx! {
        div { class: "pb-6",
            PageHeader {
                title: "Connections",
                subtitle: "Active connection tracking table",
                RefreshBtn { onclick: move |_| { conns.restart(); } }
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
                        option { value: "closing", "Closing" }
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
                    {format!("Showing {}\u{2013}{} of {}", current_page() * PAGE_SIZE + 1, (current_page() * PAGE_SIZE + entries.len()).min(total), total)}
                }
            }

            // Connections table
            DataTable {
                thead { class: "bg-slate-900/80",
                    tr {
                        th { class: TH_CLASS, "Protocol" }
                        th { class: TH_CLASS, "Source" }
                        th { class: TH_CLASS, "Destination" }
                        th { class: TH_CLASS, "State" }
                        th { class: TH_CLASS, "Packets" }
                        th { class: TH_CLASS, "Bytes" }
                    }
                }
                tbody {
                    match &*conns.read() {
                        Some(Ok(data)) if !data.entries.is_empty() => rsx! {
                            for (i, conn) in data.entries.iter().enumerate() {
                                tr { class: TR_CLASS,
                                    key: "{i}",
                                    td { class: TD_CLASS,
                                        Badge { color: Color::Slate, label: "{conn.protocol}" }
                                    }
                                    td { class: "{TD_CLASS} text-slate-300 font-mono",
                                        span { class: "text-slate-300", "{conn.src_ip}" }
                                        span { class: "text-slate-600", ":{conn.src_port}" }
                                    }
                                    td { class: "{TD_CLASS} text-slate-300 font-mono",
                                        span { class: "text-slate-300", "{conn.dst_ip}" }
                                        span { class: "text-slate-600", ":{conn.dst_port}" }
                                    }
                                    td { class: TD_CLASS,
                                        match conn.state {
                                            ConnState::Established => rsx! { Badge { color: Color::Emerald, label: "Established" } },
                                            ConnState::New => rsx! { Badge { color: Color::Amber, label: "New" } },
                                            ConnState::Related => rsx! { Badge { color: Color::Blue, label: "Related" } },
                                            ConnState::Invalid => rsx! { Badge { color: Color::Red, label: "Invalid" } },
                                            ConnState::Closing => rsx! { Badge { color: Color::Slate, label: "Closing" } },
                                        }
                                    }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono",
                                        span { class: "text-emerald-400", "{conn.packets_in}" }
                                        span { class: "text-slate-600", " / " }
                                        span { class: "text-blue-400", "{conn.packets_out}" }
                                    }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono",
                                        {format_bytes(conn.bytes_in)}
                                        span { class: "text-slate-600", " / " }
                                        {format_bytes(conn.bytes_out)}
                                    }
                                }
                            }
                        },
                        Some(Ok(_)) => rsx! {
                            TableEmpty { colspan: 6, message: "No active connections" }
                        },
                        Some(Err(e)) => rsx! {
                            TableError { colspan: 6, message: "Failed to load: {e}" }
                        },
                        None => rsx! {
                            TableLoading { colspan: 6 }
                        },
                    }
                }
            }

            // Pagination controls
            Pagination { current: current_page(), total_pages, on_change: move |p| current_page.set(p) }
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
