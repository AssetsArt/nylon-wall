use dioxus::prelude::*;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Policies() -> Element {
    let zones = use_resource(|| async {
        api_client::get::<Vec<Zone>>("/zones").await
    });
    let policies = use_resource(|| async {
        api_client::get::<Vec<NetworkPolicy>>("/policies").await
    });

    rsx! {
        div { class: "page",
            h1 { "Network Policies" }

            // Zones section
            section { class: "section",
                h2 { "Zones" }
                match &*zones.read() {
                    Some(Ok(list)) => rsx! {
                        div { class: "stats-grid",
                            for zone in list.iter() {
                                div { class: "card", key: "{zone.id}",
                                    h3 { "{zone.name}" }
                                    p { "Interfaces: {zone.interfaces.join(\", \")}" }
                                    p { "Default: "
                                        span {
                                            class: match zone.default_policy {
                                                RuleAction::Allow => "badge badge-success",
                                                RuleAction::Drop => "badge badge-error",
                                                _ => "badge",
                                            },
                                            match zone.default_policy {
                                                RuleAction::Allow => "ALLOW",
                                                RuleAction::Drop => "DROP",
                                                RuleAction::Log => "LOG",
                                                RuleAction::RateLimit => "RATE LIMIT",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if list.is_empty() {
                            p { class: "empty-state", "No zones configured." }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "error", "Error: {e}" } },
                    None => rsx! { p { "Loading..." } },
                }
            }

            // Policies section
            section { class: "section",
                h2 { "Inter-Zone Policies" }
                match &*policies.read() {
                    Some(Ok(list)) => rsx! {
                        table { class: "data-table",
                            thead {
                                tr {
                                    th { "Name" }
                                    th { "From Zone" }
                                    th { "To Zone" }
                                    th { "Protocol" }
                                    th { "Action" }
                                    th { "Status" }
                                }
                            }
                            tbody {
                                for policy in list.iter() {
                                    tr { key: "{policy.id}",
                                        td { "{policy.name}" }
                                        td { "{policy.from_zone}" }
                                        td { "{policy.to_zone}" }
                                        td { {policy.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                        td {
                                            span {
                                                class: match policy.action {
                                                    RuleAction::Allow => "badge badge-success",
                                                    RuleAction::Drop => "badge badge-error",
                                                    _ => "badge badge-warning",
                                                },
                                                match policy.action {
                                                    RuleAction::Allow => "ALLOW",
                                                    RuleAction::Drop => "DROP",
                                                    RuleAction::Log => "LOG",
                                                    RuleAction::RateLimit => "RATE LIMIT",
                                                }
                                            }
                                        }
                                        td {
                                            span {
                                                class: if policy.enabled { "badge badge-success" } else { "badge badge-muted" },
                                                if policy.enabled { "Active" } else { "Inactive" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if list.is_empty() {
                            p { class: "empty-state", "No policies configured." }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "error", "Error: {e}" } },
                    None => rsx! { p { "Loading..." } },
                }
            }
        }
    }
}
