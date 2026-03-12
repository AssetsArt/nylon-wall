use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Routes() -> Element {
    let mut routes = use_resource(|| async {
        api_client::get::<Vec<Route>>("/routes").await
    });
    let mut show_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "Routing Table" }
                    p { class: "text-sm text-slate-400 mt-1", "Static routes and network paths" }
                }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ Add Route" }
                }
            }

            if let Some(err) = error_msg() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 flex items-center justify-between",
                    span { "{err}" }
                    button {
                        class: "text-red-400 hover:text-red-300",
                        onclick: move |_| error_msg.set(None),
                        Icon { width: 14, height: 14, icon: LdX }
                    }
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

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Gateway" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Interface" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Metric" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Table" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*routes.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for route in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{route.id}",
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono font-medium", "{route.destination}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono", {route.gateway.clone().unwrap_or("\u{2014}".to_string())} }
                                        td { class: "px-5 py-3 text-sm text-slate-400", "{route.interface}" }
                                        td { class: "px-5 py-3 text-sm text-cyan-400 font-mono", "{route.metric}" }
                                        td { class: "px-5 py-3 text-sm text-slate-500", "{route.table}" }
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: if route.enabled {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                                } else {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20"
                                                },
                                                if route.enabled { "Active" } else { "Inactive" }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let id = route.id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/routes/{}", id)).await {
                                                                    Ok(_) => routes.restart(),
                                                                    Err(e) => error_msg.set(Some(e)),
                                                                }
                                                            });
                                                        },
                                                        Icon { width: 13, height: 13, icon: LdTrash2 }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "7", "No routes configured" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "7", "Failed to load routes: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "7", "Loading..." } }
                            },
                        }
                    }
                }
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
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let route = Route {
            id: 0,
            destination: destination(),
            gateway: if gateway().is_empty() { None } else { Some(gateway()) },
            interface: interface(),
            metric: metric().parse().unwrap_or(100),
            table: 254,
            enabled: true,
        };
        spawn(async move {
            match api_client::post::<Route, Route>("/routes", &route).await {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Add Static Route" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Destination (CIDR)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "10.0.0.0/8", value: "{destination}",
                        oninput: move |e| destination.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Gateway" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.1", value: "{gateway}",
                        oninput: move |e| gateway.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Interface" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "eth0", value: "{interface}",
                        oninput: move |e| interface.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Metric" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "number", value: "{metric}",
                        oninput: move |e| metric.set(e.value()),
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Adding..." } else { "Add Route" }
            }
        }
    }
}
