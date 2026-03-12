use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;
use nylon_wall_common::conntrack::ConnState;

#[component]
pub fn Connections() -> Element {
    let mut conns = use_resource(|| async {
        api_client::get::<Vec<ConntrackInfo>>("/conntrack").await
    });

    let conn_list = match &*conns.read() {
        Some(Ok(list)) => list.clone(),
        _ => vec![],
    };
    let total = conn_list.len();
    let established = conn_list.iter().filter(|c| c.state == ConnState::Established).count();
    let new_count = conn_list.iter().filter(|c| c.state == ConnState::New).count();

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
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for (i, conn) in list.iter().enumerate() {
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
                                            span { class: "text-emerald-400", "{conn.bytes_in}" }
                                            span { class: "text-slate-600", " / " }
                                            span { class: "text-blue-400", "{conn.bytes_out}" }
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
        }
    }
}
