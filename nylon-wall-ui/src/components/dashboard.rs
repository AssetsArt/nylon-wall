use dioxus::prelude::*;
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

    rsx! {
        div { class: "page",
            h1 { "Dashboard" }
            div { class: "stats-grid",
                // System Status card
                div { class: "card",
                    h3 { "System Status" }
                    match &*status.read() {
                        Some(Ok(s)) => rsx! {
                            p { "Version: {s.version}" }
                            p { "eBPF: " span { class: if s.ebpf_loaded { "badge badge-success" } else { "badge badge-error" },
                                if s.ebpf_loaded { "Loaded" } else { "Not Loaded" }
                            }}
                        },
                        Some(Err(e)) => rsx! { p { class: "error", "Error: {e}" } },
                        None => rsx! { p { "Loading..." } },
                    }
                }
                // Rules count card
                div { class: "card",
                    h3 { "Firewall Rules" }
                    match &*rules.read() {
                        Some(Ok(r)) => rsx! {
                            p { class: "stat-number", "{r.len()}" }
                            p { class: "stat-label", "Total Rules" }
                            p { "{r.iter().filter(|r| r.enabled).count()} active" }
                        },
                        Some(Err(e)) => rsx! { p { class: "error", "Error: {e}" } },
                        None => rsx! { p { "Loading..." } },
                    }
                }
                // Connections card (placeholder)
                div { class: "card",
                    h3 { "Connections" }
                    p { class: "stat-number", "\u{2014}" }
                    p { class: "stat-label", "Active Connections" }
                }
                // Traffic card (placeholder)
                div { class: "card",
                    h3 { "Traffic" }
                    p { class: "stat-label", "Traffic monitoring coming soon" }
                }
            }
        }
    }
}
