use dioxus::prelude::*;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Routes() -> Element {
    let mut routes = use_resource(|| async {
        api_client::get::<Vec<Route>>("/routes").await
    });
    let mut show_form = use_signal(|| false);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                h1 { "Routing Table" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ Add Route" }
                }
            }

            if show_form() {
                RouteForm {
                    on_saved: move |_| {
                        show_form.set(false);
                        routes.restart();
                    }
                }
            }

            match &*routes.read() {
                Some(Ok(list)) => rsx! {
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Destination" }
                                th { "Gateway" }
                                th { "Interface" }
                                th { "Metric" }
                                th { "Table" }
                                th { "Status" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            for route in list.iter() {
                                tr { key: "{route.id}",
                                    td { "{route.destination}" }
                                    td { {route.gateway.clone().unwrap_or("\u{2014}".to_string())} }
                                    td { "{route.interface}" }
                                    td { "{route.metric}" }
                                    td { "{route.table}" }
                                    td {
                                        span {
                                            class: if route.enabled { "badge badge-success" } else { "badge badge-muted" },
                                            if route.enabled { "Active" } else { "Inactive" }
                                        }
                                    }
                                    td {
                                        {
                                            let id = route.id;
                                            rsx! {
                                                button {
                                                    class: "btn btn-sm btn-danger",
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            let _ = api_client::delete(&format!("/routes/{}", id)).await;
                                                            routes.restart();
                                                        });
                                                    },
                                                    "Delete"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if list.is_empty() {
                        p { class: "empty-state", "No routes configured." }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Failed to load routes: {e}" } },
                None => rsx! { p { "Loading..." } },
            }
        }
    }
}

#[component]
fn RouteForm(on_saved: EventHandler<()>) -> Element {
    let mut destination = use_signal(|| String::new());
    let mut gateway = use_signal(|| String::new());
    let mut interface = use_signal(|| String::new());
    let mut metric = use_signal(|| "100".to_string());

    let on_submit = move |_| {
        let route = Route {
            id: 0,
            destination: destination(),
            gateway: if gateway().is_empty() { None } else { Some(gateway()) },
            interface: interface(),
            metric: metric().parse().unwrap_or(100),
            table: 254, // main table
            enabled: true,
        };
        spawn(async move {
            match api_client::post::<Route, Route>("/routes", &route).await {
                Ok(_) => on_saved.call(()),
                Err(e) => tracing::error!("Failed to create route: {}", e),
            }
        });
    };

    rsx! {
        div { class: "card form-card",
            h3 { "Add Static Route" }
            div { class: "form-grid",
                div { class: "form-group",
                    label { "Destination (CIDR)" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 10.0.0.0/8",
                        value: "{destination}",
                        oninput: move |e| destination.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Gateway" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 192.168.1.1",
                        value: "{gateway}",
                        oninput: move |e| gateway.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Interface" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. eth0",
                        value: "{interface}",
                        oninput: move |e| interface.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Metric" }
                    input {
                        r#type: "number",
                        value: "{metric}",
                        oninput: move |e| metric.set(e.value()),
                    }
                }
            }
            div { class: "form-actions",
                button { class: "btn btn-primary", onclick: on_submit, "Add Route" }
            }
        }
    }
}
