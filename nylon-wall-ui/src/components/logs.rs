use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Logs() -> Element {
    let mut logs = use_resource(|| async {
        api_client::get::<Vec<PacketLog>>("/logs").await
    });

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
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "No packet logs available" } }
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
