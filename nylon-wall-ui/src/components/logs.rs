use dioxus::prelude::*;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Logs() -> Element {
    let mut logs = use_resource(|| async {
        api_client::get::<Vec<PacketLog>>("/logs").await
    });

    rsx! {
        div { class: "page",
            div { class: "page-header",
                h1 { "Packet Logs" }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| { logs.restart(); },
                    "Refresh"
                }
            }

            match &*logs.read() {
                Some(Ok(list)) => rsx! {
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Time" }
                                th { "Source" }
                                th { "Destination" }
                                th { "Protocol" }
                                th { "Action" }
                                th { "Rule" }
                                th { "Interface" }
                                th { "Bytes" }
                            }
                        }
                        tbody {
                            for (i, log) in list.iter().enumerate() {
                                tr { key: "{i}",
                                    td { "{log.timestamp}" }
                                    td { "{log.src_ip}:{log.src_port}" }
                                    td { "{log.dst_ip}:{log.dst_port}" }
                                    td { "{log.protocol}" }
                                    td {
                                        span {
                                            class: if log.action == "DROP" { "badge badge-error" } else { "badge badge-success" },
                                            "{log.action}"
                                        }
                                    }
                                    td { "#{log.rule_id}" }
                                    td { "{log.interface}" }
                                    td { "{log.bytes}" }
                                }
                            }
                        }
                    }
                    if list.is_empty() {
                        p { class: "empty-state", "No packet logs available." }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Failed to load logs: {e}" } },
                None => rsx! { p { "Loading logs..." } },
            }
        }
    }
}
