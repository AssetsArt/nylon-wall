use super::ui::*;
use super::use_refresh_trigger;
use crate::api_client;
use crate::models::*;
use crate::ws_client::use_ws_events;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

const PAGE_SIZE: usize = 25;

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct PaginatedLogs {
    total: usize,
    offset: usize,
    limit: usize,
    entries: Vec<PacketLog>,
}

fn protocol_color(proto: &str) -> Color {
    match proto.to_uppercase().as_str() {
        "TCP" => Color::Blue,
        "UDP" => Color::Violet,
        "ICMP" => Color::Cyan,
        _ => Color::Slate,
    }
}

fn action_color(action: &str) -> Color {
    match action.to_uppercase().as_str() {
        "DROP" => Color::Red,
        "LOG" => Color::Amber,
        _ => Color::Emerald,
    }
}

#[component]
pub fn Logs() -> Element {
    let mut current_page = use_signal(|| 0usize);
    let mut filter_src = use_signal(String::new);
    let mut filter_dst = use_signal(String::new);
    let mut filter_proto = use_signal(String::new);
    let mut filter_action = use_signal(String::new);

    let page = current_page();
    let src = filter_src();
    let dst = filter_dst();
    let proto = filter_proto();
    let action = filter_action();

    let ws = use_ws_events();
    let refresh = use_refresh_trigger();
    let mut prev = use_signal(|| (refresh(), ws.logs()));

    let mut logs = use_resource(use_reactive!(
        |(page, src, dst, proto, action)| async move {
            let offset = page * PAGE_SIZE;
            let mut params = vec![format!("limit={}", PAGE_SIZE), format!("offset={}", offset)];
            if !src.is_empty() {
                params.push(format!("src_ip={}", src));
            }
            if !dst.is_empty() {
                params.push(format!("dst_ip={}", dst));
            }
            if !proto.is_empty() {
                params.push(format!("protocol={}", proto));
            }
            if !action.is_empty() {
                params.push(format!("action={}", action));
            }
            let query = params.join("&");
            api_client::get::<PaginatedLogs>(&format!("/logs?{}", query)).await
        }
    ));

    use_effect(move || {
        let current = (refresh(), ws.logs());
        if current != prev() {
            prev.set(current);
            logs.restart();
        }
    });

    let (entries, total) = match &*logs.read() {
        Some(Ok(data)) => (data.entries.clone(), data.total),
        _ => (vec![], 0),
    };

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(PAGE_SIZE)
    };

    rsx! {
        div {
            PageHeader {
                title: "Packet Logs",
                subtitle: "Real-time packet inspection and audit trail",
                RefreshBtn { onclick: move |_| { logs.restart(); } }
            }

            // Filter bar
            FormCard {
                class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4 mb-6",
                div { class: "flex items-center gap-2 mb-3",
                    Icon { width: 14, height: 14, icon: LdFilter, class: "text-slate-500" }
                    span { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider", "Filters" }
                }
                div { class: "grid grid-cols-2 sm:grid-cols-4 gap-3",
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Source IP" }
                        input {
                            class: INPUT_CLASS,
                            placeholder: "e.g. 192.168.1.100",
                            value: "{filter_src}",
                            oninput: move |e| { filter_src.set(e.value()); current_page.set(0); },
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Destination IP" }
                        input {
                            class: INPUT_CLASS,
                            placeholder: "e.g. 10.0.0.1",
                            value: "{filter_dst}",
                            oninput: move |e| { filter_dst.set(e.value()); current_page.set(0); },
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Protocol" }
                        select {
                            class: SELECT_CLASS,
                            value: "{filter_proto}",
                            onchange: move |e| { filter_proto.set(e.value()); current_page.set(0); },
                            option { value: "", "All" }
                            option { value: "TCP", "TCP" }
                            option { value: "UDP", "UDP" }
                            option { value: "ICMP", "ICMP" }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Action" }
                        select {
                            class: SELECT_CLASS,
                            value: "{filter_action}",
                            onchange: move |e| { filter_action.set(e.value()); current_page.set(0); },
                            option { value: "", "All" }
                            option { value: "ALLOW", "Allow" }
                            option { value: "DROP", "Drop" }
                            option { value: "LOG", "Log" }
                        }
                    }
                }
                if !filter_src().is_empty() || !filter_dst().is_empty() || !filter_proto().is_empty() || !filter_action().is_empty() {
                    div { class: "mt-3 flex items-center gap-2",
                        button {
                            class: "px-2 py-1 rounded text-[11px] font-medium text-slate-500 hover:text-slate-300 hover:bg-slate-800/50 transition-colors",
                            onclick: move |_| {
                                filter_src.set(String::new());
                                filter_dst.set(String::new());
                                filter_proto.set(String::new());
                                filter_action.set(String::new());
                                current_page.set(0);
                            },
                            "Clear all filters"
                        }
                    }
                }
            }

            // Showing X–Y of Z
            div { class: "flex items-center justify-between mb-3",
                div { class: "text-xs text-slate-500",
                    {format!("Showing {}\u{2013}{} of {}", page * PAGE_SIZE + 1, (page * PAGE_SIZE + entries.len()).min(total), total)}
                }
            }

            DataTable {
                thead { class: "bg-slate-900/80",
                    tr {
                        th { class: TH_CLASS, "Time" }
                        th { class: TH_CLASS, "Source" }
                        th { class: TH_CLASS, "Destination" }
                        th { class: TH_CLASS, "Protocol" }
                        th { class: TH_CLASS, "Action" }
                        th { class: TH_CLASS, "Rule" }
                        th { class: TH_CLASS, "Interface" }
                        th { class: TH_CLASS, "Bytes" }
                    }
                }
                tbody {
                    match &*logs.read() {
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
                                        Badge { color: protocol_color(&log.protocol), label: log.protocol.clone() }
                                    }
                                    td { class: TD_CLASS,
                                        Badge { color: action_color(&log.action), label: log.action.clone() }
                                    }
                                    td { class: "{TD_CLASS} text-slate-500 font-mono", "#{log.rule_id}" }
                                    td { class: "{TD_CLASS} text-slate-400", "{log.interface}" }
                                    td { class: "{TD_CLASS} text-cyan-400 font-mono", "{log.bytes}" }
                                }
                            }
                        },
                        Some(Ok(_)) => rsx! {
                            TableEmpty { colspan: 8, message: "No packet logs match the current filters" }
                        },
                        Some(Err(e)) => rsx! {
                            TableError { colspan: 8, message: format!("Failed to load logs: {e}") }
                        },
                        None => rsx! {
                            TableLoading { colspan: 8 }
                        },
                    }
                }
            }

            Pagination { current: current_page(), total_pages, on_change: move |p| current_page.set(p) }
        }
    }
}
