use dioxus::prelude::*;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Settings() -> Element {
    let status = use_resource(|| async {
        api_client::get::<SystemStatus>("/system/status").await
    });

    rsx! {
        div { class: "page",
            h1 { "Settings" }

            section { class: "section",
                h2 { "System Information" }
                div { class: "card",
                    match &*status.read() {
                        Some(Ok(s)) => rsx! {
                            div { class: "info-grid",
                                div { class: "info-row",
                                    span { class: "info-label", "Version" }
                                    span { class: "info-value", "{s.version}" }
                                }
                                div { class: "info-row",
                                    span { class: "info-label", "eBPF Status" }
                                    span { class: "info-value",
                                        span {
                                            class: if s.ebpf_loaded { "badge badge-success" } else { "badge badge-error" },
                                            if s.ebpf_loaded { "Loaded" } else { "Not Loaded" }
                                        }
                                    }
                                }
                                div { class: "info-row",
                                    span { class: "info-label", "Uptime" }
                                    span { class: "info-value", "{s.uptime_seconds}s" }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! { p { class: "error", "Error: {e}" } },
                        None => rsx! { p { "Loading..." } },
                    }
                }
            }

            section { class: "section",
                h2 { "Backup & Restore" }
                div { class: "card",
                    div { class: "form-actions",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| {
                                spawn(async move {
                                    match api_client::post::<(), serde_json::Value>("/system/backup", &()).await {
                                        Ok(_) => tracing::info!("Backup created"),
                                        Err(e) => tracing::error!("Backup failed: {}", e),
                                    }
                                });
                            },
                            "Export Configuration"
                        }
                        button {
                            class: "btn btn-secondary",
                            "Import Configuration"
                        }
                    }
                }
            }
        }
    }
}
