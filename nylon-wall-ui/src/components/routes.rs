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

            // Policy Routes section
            PolicyRoutes {}

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

// === Policy Routes ===

#[component]
pub fn PolicyRoutes() -> Element {
    let mut policy_routes = use_resource(|| async {
        api_client::get::<Vec<PolicyRoute>>("/routes/policy").await
    });
    let mut show_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div { class: "mt-8",
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "Policy Routes" }
                    p { class: "text-sm text-slate-400 mt-1", "Route traffic based on source, destination, port, or protocol" }
                }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-purple-500/10 text-purple-400 border border-purple-500/20 hover:bg-purple-500/20 transition-colors",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ Add Policy Route" }
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
                PolicyRouteForm {
                    on_saved: move |_| {
                        show_form.set(false);
                        policy_routes.restart();
                    }
                }
            }

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Priority" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Source" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Port" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Table" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*policy_routes.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for pr in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{pr.id}",
                                        td { class: "px-5 py-3 text-sm text-purple-400 font-mono font-medium", "{pr.priority}" }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            {pr.src_ip.clone().unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            {pr.dst_ip.clone().unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono",
                                            {pr.src_port.map(|p| format!("{}:{}", p.start, p.end)).unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400",
                                            {pr.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-cyan-400 font-mono", "{pr.route_table}" }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let id = pr.id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/routes/policy/{}", id)).await {
                                                                    Ok(_) => policy_routes.restart(),
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
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "7", "No policy routes configured" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "7", "Failed to load: {e}" } }
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
fn PolicyRouteForm(on_saved: EventHandler<()>) -> Element {
    let mut src_ip = use_signal(|| String::new());
    let mut dst_ip = use_signal(|| String::new());
    let mut src_port = use_signal(|| String::new());
    let mut protocol = use_signal(|| "any".to_string());
    let mut route_table = use_signal(|| "100".to_string());
    let mut priority = use_signal(|| "1000".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);

        let port_val = src_port();
        let parsed_port = if port_val.is_empty() {
            None
        } else {
            let parts: Vec<&str> = port_val.split(':').collect();
            match parts.len() {
                1 => parts[0].parse::<u16>().ok().map(|p| PortRange { start: p, end: p }),
                2 => {
                    let s = parts[0].parse::<u16>().unwrap_or(0);
                    let e = parts[1].parse::<u16>().unwrap_or(0);
                    Some(PortRange { start: s, end: e })
                },
                _ => None,
            }
        };

        let parsed_protocol = match protocol().as_str() {
            "tcp" => Some(Protocol::TCP),
            "udp" => Some(Protocol::UDP),
            "icmp" => Some(Protocol::ICMP),
            _ => None,
        };

        let pr = PolicyRoute {
            id: 0,
            src_ip: if src_ip().is_empty() { None } else { Some(src_ip()) },
            dst_ip: if dst_ip().is_empty() { None } else { Some(dst_ip()) },
            src_port: parsed_port,
            protocol: parsed_protocol,
            route_table: route_table().parse().unwrap_or(100),
            priority: priority().parse().unwrap_or(1000),
        };

        spawn(async move {
            match api_client::post::<PolicyRoute, PolicyRoute>("/routes/policy", &pr).await {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-purple-500/20 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Add Policy Route" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Source IP (CIDR)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-purple-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{src_ip}",
                        oninput: move |e| src_ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Destination IP (CIDR)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-purple-500/60 transition-colors",
                        r#type: "text", placeholder: "10.0.0.0/8", value: "{dst_ip}",
                        oninput: move |e| dst_ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Source Port (or range start:end)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-purple-500/60 transition-colors",
                        r#type: "text", placeholder: "80 or 8000:9000", value: "{src_port}",
                        oninput: move |e| src_port.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Protocol" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-purple-500/60 transition-colors",
                        value: "{protocol}",
                        onchange: move |e| protocol.set(e.value()),
                        option { value: "any", "Any" }
                        option { value: "tcp", "TCP" }
                        option { value: "udp", "UDP" }
                        option { value: "icmp", "ICMP" }
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Route Table" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-purple-500/60 transition-colors",
                        r#type: "number", value: "{route_table}",
                        oninput: move |e| route_table.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Priority" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-purple-500/60 transition-colors",
                        r#type: "number", value: "{priority}",
                        oninput: move |e| priority.set(e.value()),
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-purple-500/10 text-purple-400 border border-purple-500/20 hover:bg-purple-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Adding..." } else { "Add Policy Route" }
            }
        }
    }
}
