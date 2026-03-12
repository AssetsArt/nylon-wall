use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Logs() -> Element {
    let mut filter_src = use_signal(|| String::new());
    let mut filter_dst = use_signal(|| String::new());
    let mut filter_proto = use_signal(|| String::new());
    let mut filter_action = use_signal(|| String::new());

    let src = filter_src();
    let dst = filter_dst();
    let proto = filter_proto();
    let action = filter_action();

    let mut logs = use_resource(use_reactive!(|(src, dst, proto, action)| async move {
        let mut params = vec!["limit=200".to_string()];
        if !src.is_empty() { params.push(format!("src_ip={}", src)); }
        if !dst.is_empty() { params.push(format!("dst_ip={}", dst)); }
        if !proto.is_empty() { params.push(format!("protocol={}", proto)); }
        if !action.is_empty() { params.push(format!("action={}", action)); }
        let query = params.join("&");
        api_client::get::<Vec<PacketLog>>(&format!("/logs?{}", query)).await
    }));

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "Packet Logs" }
                    p { class: "text-sm text-slate-400 mt-1", "Real-time packet inspection and audit trail" }
                }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors flex items-center gap-1.5",
                    onclick: move |_| { logs.restart(); },
                    Icon { width: 13, height: 13, icon: LdRefreshCw }
                    "Refresh"
                }
            }

            // Filter bar
            div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4 mb-6",
                div { class: "flex items-center gap-2 mb-3",
                    Icon { width: 14, height: 14, icon: LdFilter, class: "text-slate-500" }
                    span { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider", "Filters" }
                }
                div { class: "grid grid-cols-2 sm:grid-cols-4 gap-3",
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Source IP" }
                        input {
                            class: "w-full rounded-lg bg-slate-800/50 border border-slate-700/50 px-3 py-1.5 text-sm text-slate-300 placeholder-slate-600 focus:outline-none focus:border-blue-500/50",
                            placeholder: "e.g. 192.168.1.100",
                            value: "{filter_src}",
                            oninput: move |e| filter_src.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Destination IP" }
                        input {
                            class: "w-full rounded-lg bg-slate-800/50 border border-slate-700/50 px-3 py-1.5 text-sm text-slate-300 placeholder-slate-600 focus:outline-none focus:border-blue-500/50",
                            placeholder: "e.g. 10.0.0.1",
                            value: "{filter_dst}",
                            oninput: move |e| filter_dst.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Protocol" }
                        select {
                            class: "w-full rounded-lg bg-slate-800/50 border border-slate-700/50 px-3 py-1.5 text-sm text-slate-300 focus:outline-none focus:border-blue-500/50",
                            value: "{filter_proto}",
                            onchange: move |e| filter_proto.set(e.value()),
                            option { value: "", "All" }
                            option { value: "TCP", "TCP" }
                            option { value: "UDP", "UDP" }
                            option { value: "ICMP", "ICMP" }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-medium text-slate-500 mb-1", "Action" }
                        select {
                            class: "w-full rounded-lg bg-slate-800/50 border border-slate-700/50 px-3 py-1.5 text-sm text-slate-300 focus:outline-none focus:border-blue-500/50",
                            value: "{filter_action}",
                            onchange: move |e| filter_action.set(e.value()),
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
                            },
                            "Clear all filters"
                        }
                    }
                }
            }

            // Log count
            if let Some(Ok(list)) = &*logs.read() {
                div { class: "mb-3 text-xs text-slate-500",
                    "Showing {list.len()} entries"
                }
            }

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Time" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Source" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Action" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Rule" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Interface" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Bytes" }
                        }
                    }
                    tbody {
                        match &*logs.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for (i, log) in list.iter().enumerate() {
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
                                                class: if log.action == "DROP" {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20"
                                                } else {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                                },
                                                "{log.action}"
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-500 font-mono", "#{log.rule_id}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400", "{log.interface}" }
                                        td { class: "px-5 py-3 text-sm text-cyan-400 font-mono", "{log.bytes}" }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "No packet logs match the current filters" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "8", "Failed to load logs: {e}" } }
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
