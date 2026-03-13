use super::ConfirmModal;
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;

#[component]
pub fn Routes() -> Element {
    let mut routes = use_resource(|| async { api_client::get::<Vec<Route>>("/routes").await });
    let mut editing = use_signal(|| None::<(bool, Route)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(u32, String)>);

    rsx! {
        div { class: "pb-6",
            PageHeader {
                title: "Routing Table".to_string(),
                subtitle: "Static routes and network paths".to_string(),
                Btn {
                    color: Color::Blue,
                    label: if editing().is_some() { "Cancel".to_string() } else { "+ Add Route".to_string() },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((false, Route {
                                id: 0, destination: String::new(), gateway: None,
                                interface: String::new(), metric: 100, table: 254, enabled: true,
                            })));
                        }
                    },
                }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            if let Some((is_edit, route)) = editing() {
                RouteForm {
                    key: "{route.id}",
                    is_edit: is_edit,
                    editing: route,
                    on_saved: move |_| {
                        editing.set(None);
                        routes.restart();
                    }
                }
            }

            if let Some((del_id, del_dest)) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Route".to_string(),
                    message: format!("Are you sure you want to delete route to \"{}\"? This action cannot be undone.", del_dest),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/routes/{}", del_id)).await {
                                Ok(_) => routes.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete.set(None); },
                }
            }

            div { class: "mb-8",
                DataTable {
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: TH_CLASS, "Destination" }
                            th { class: TH_CLASS, "Gateway" }
                            th { class: TH_CLASS, "Interface" }
                            th { class: TH_CLASS, "Metric" }
                            th { class: TH_CLASS, "Table" }
                            th { class: TH_CLASS, "Status" }
                            th { class: TH_CLASS, "" }
                        }
                    }
                    tbody {
                        match &*routes.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for route in list.iter() {
                                    tr { class: TR_CLASS,
                                        key: "{route.id}",
                                        td { class: "{TD_CLASS} text-slate-300 font-mono font-medium", "{route.destination}" }
                                        td { class: "{TD_CLASS} text-slate-400 font-mono", {route.gateway.clone().unwrap_or("\u{2014}".to_string())} }
                                        td { class: "{TD_CLASS} text-slate-400", "{route.interface}" }
                                        td { class: "{TD_CLASS} text-cyan-400 font-mono", "{route.metric}" }
                                        td { class: "{TD_CLASS} text-slate-500", "{route.table}" }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if route.enabled { Color::Emerald } else { Color::Slate },
                                                label: if route.enabled { "Active".to_string() } else { "Inactive".to_string() },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            {
                                                let route_clone = route.clone();
                                                let id = route.id;
                                                let dest = route.destination.clone();
                                                rsx! {
                                                    div { class: "flex items-center gap-1",
                                                        EditBtn {
                                                            onclick: move |_| {
                                                                editing.set(Some((true, route_clone.clone())));
                                                            },
                                                        }
                                                        DeleteBtn {
                                                            onclick: move |_| {
                                                                confirm_delete.set(Some((id, dest.clone())));
                                                            },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                TableEmpty { colspan: 7, message: "No routes configured".to_string() }
                            },
                            Some(Err(e)) => rsx! {
                                TableError { colspan: 7, message: format!("Failed to load routes: {e}") }
                            },
                            None => rsx! {
                                TableLoading { colspan: 7 }
                            },
                        }
                    }
                }
            }

            // Policy Routes section
            PolicyRoutes {}
        }
    }
}

#[component]
fn RouteForm(is_edit: bool, editing: Route, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut destination = use_signal(|| editing.destination.clone());
    let mut gateway = use_signal(|| editing.gateway.clone().unwrap_or_default());
    let mut interface = use_signal(|| editing.interface.clone());
    let mut metric = use_signal(|| editing.metric.to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let route = Route {
            id: edit_id,
            destination: destination(),
            gateway: if gateway().is_empty() {
                None
            } else {
                Some(gateway())
            },
            interface: interface(),
            metric: metric().parse().unwrap_or(100),
            table: 254,
            enabled: true,
        };
        spawn(async move {
            let result = if is_edit {
                api_client::put::<Route, Route>(&format!("/routes/{}", edit_id), &route).await
            } else {
                api_client::post::<Route, Route>("/routes", &route).await
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        FormCard {
            h3 { class: "text-sm font-semibold text-white mb-4",
                if is_edit { "Edit Route" } else { "Add Static Route" }
            }
            if let Some(err) = error() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error.set(None),
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                FormField { label: "Destination (CIDR)".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "10.0.0.0/8", value: "{destination}",
                        oninput: move |e| destination.set(e.value()),
                    }
                }
                FormField { label: "Gateway".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.1", value: "{gateway}",
                        oninput: move |e| gateway.set(e.value()),
                    }
                }
                InterfaceSelect {
                    value: interface(),
                    onchange: move |v| interface.set(v),
                }
                FormField { label: "Metric".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", value: "{metric}",
                        oninput: move |e| metric.set(e.value()),
                    }
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Adding...".to_string() }
                } else {
                    if is_edit { "Save Route".to_string() } else { "Add Route".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}

// === Policy Routes ===

#[component]
pub fn PolicyRoutes() -> Element {
    let mut policy_routes =
        use_resource(|| async { api_client::get::<Vec<PolicyRoute>>("/routes/policy").await });
    let mut editing = use_signal(|| None::<(bool, PolicyRoute)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<u32>);

    rsx! {
        div { class: "mt-8",
            PageHeader {
                title: "Policy Routes".to_string(),
                subtitle: "Route traffic based on source, destination, port, or protocol".to_string(),
                Btn {
                    color: Color::Purple,
                    label: if editing().is_some() { "Cancel".to_string() } else { "+ Add Policy Route".to_string() },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((false, PolicyRoute {
                                id: 0, src_ip: None, dst_ip: None, src_port: None,
                                protocol: None, route_table: 100, priority: 1000,
                            })));
                        }
                    },
                }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            if let Some((is_edit, pr)) = editing() {
                PolicyRouteForm {
                    key: "{pr.id}",
                    is_edit: is_edit,
                    editing: pr,
                    on_saved: move |_| {
                        editing.set(None);
                        policy_routes.restart();
                    }
                }
            }

            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Policy Route".to_string(),
                    message: format!("Are you sure you want to delete policy route #{}? This action cannot be undone.", del_id),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/routes/policy/{}", del_id)).await {
                                Ok(_) => policy_routes.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete.set(None); },
                }
            }

            DataTable {
                thead { class: "bg-slate-900/80",
                    tr {
                        th { class: TH_CLASS, "Priority" }
                        th { class: TH_CLASS, "Source" }
                        th { class: TH_CLASS, "Destination" }
                        th { class: TH_CLASS, "Port" }
                        th { class: TH_CLASS, "Protocol" }
                        th { class: TH_CLASS, "Table" }
                        th { class: TH_CLASS, "" }
                    }
                }
                tbody {
                    match &*policy_routes.read() {
                        Some(Ok(list)) if !list.is_empty() => rsx! {
                            for pr in list.iter() {
                                tr { class: TR_CLASS,
                                    key: "{pr.id}",
                                    td { class: "{TD_CLASS} text-purple-400 font-mono font-medium", "{pr.priority}" }
                                    td { class: "{TD_CLASS} text-slate-300 font-mono",
                                        {pr.src_ip.clone().unwrap_or("\u{2014}".to_string())}
                                    }
                                    td { class: "{TD_CLASS} text-slate-300 font-mono",
                                        {pr.dst_ip.clone().unwrap_or("\u{2014}".to_string())}
                                    }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono",
                                        {pr.src_port.map(|p| format!("{}:{}", p.start, p.end)).unwrap_or("\u{2014}".to_string())}
                                    }
                                    td { class: "{TD_CLASS} text-slate-400",
                                        {pr.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())}
                                    }
                                    td { class: "{TD_CLASS} text-cyan-400 font-mono", "{pr.route_table}" }
                                    td { class: TD_CLASS,
                                        {
                                            let pr_clone = pr.clone();
                                            let id = pr.id;
                                            rsx! {
                                                div { class: "flex items-center gap-1",
                                                    EditBtn {
                                                        onclick: move |_| {
                                                            editing.set(Some((true, pr_clone.clone())));
                                                        },
                                                    }
                                                    DeleteBtn {
                                                        onclick: move |_| {
                                                            confirm_delete.set(Some(id));
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Ok(_)) => rsx! {
                            TableEmpty { colspan: 7, message: "No policy routes configured".to_string() }
                        },
                        Some(Err(e)) => rsx! {
                            TableError { colspan: 7, message: format!("Failed to load: {e}") }
                        },
                        None => rsx! {
                            TableLoading { colspan: 7 }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn PolicyRouteForm(is_edit: bool, editing: PolicyRoute, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut src_ip = use_signal(|| editing.src_ip.clone().unwrap_or_default());
    let mut dst_ip = use_signal(|| editing.dst_ip.clone().unwrap_or_default());
    let mut src_port = use_signal(|| {
        editing.src_port.map(|p| {
            if p.start == p.end { p.start.to_string() } else { format!("{}:{}", p.start, p.end) }
        }).unwrap_or_default()
    });
    let mut protocol = use_signal(|| match editing.protocol {
        Some(Protocol::TCP) => "tcp".to_string(),
        Some(Protocol::UDP) => "udp".to_string(),
        Some(Protocol::ICMP) => "icmp".to_string(),
        _ => "any".to_string(),
    });
    let mut route_table = use_signal(|| editing.route_table.to_string());
    let mut priority = use_signal(|| editing.priority.to_string());
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
                1 => parts[0]
                    .parse::<u16>()
                    .ok()
                    .map(|p| PortRange { start: p, end: p }),
                2 => {
                    let s = parts[0].parse::<u16>().unwrap_or(0);
                    let e = parts[1].parse::<u16>().unwrap_or(0);
                    Some(PortRange { start: s, end: e })
                }
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
            id: edit_id,
            src_ip: if src_ip().is_empty() {
                None
            } else {
                Some(src_ip())
            },
            dst_ip: if dst_ip().is_empty() {
                None
            } else {
                Some(dst_ip())
            },
            src_port: parsed_port,
            protocol: parsed_protocol,
            route_table: route_table().parse().unwrap_or(100),
            priority: priority().parse().unwrap_or(1000),
        };

        spawn(async move {
            let result = if is_edit {
                api_client::put::<PolicyRoute, PolicyRoute>(&format!("/routes/policy/{}", edit_id), &pr).await
            } else {
                api_client::post::<PolicyRoute, PolicyRoute>("/routes/policy", &pr).await
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        FormCard {
            class: "rounded-xl border border-purple-500/20 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4",
                if is_edit { "Edit Policy Route" } else { "Add Policy Route" }
            }
            if let Some(err) = error() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error.set(None),
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-4",
                FormField { label: "Source IP (CIDR)".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{src_ip}",
                        oninput: move |e| src_ip.set(e.value()),
                    }
                }
                FormField { label: "Destination IP (CIDR)".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "10.0.0.0/8", value: "{dst_ip}",
                        oninput: move |e| dst_ip.set(e.value()),
                    }
                }
                FormField { label: "Source Port (or range start:end)".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "80 or 8000:9000", value: "{src_port}",
                        oninput: move |e| src_port.set(e.value()),
                    }
                }
                FormField { label: "Protocol".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{protocol}",
                        onchange: move |e| protocol.set(e.value()),
                        option { value: "any", "Any" }
                        option { value: "tcp", "TCP" }
                        option { value: "udp", "UDP" }
                        option { value: "icmp", "ICMP" }
                    }
                }
                FormField { label: "Route Table".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", value: "{route_table}",
                        oninput: move |e| route_table.set(e.value()),
                    }
                }
                FormField { label: "Priority".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", value: "{priority}",
                        oninput: move |e| priority.set(e.value()),
                    }
                }
            }
            SubmitBtn {
                color: Color::Purple,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Adding...".to_string() }
                } else {
                    if is_edit { "Save Policy Route".to_string() } else { "Add Policy Route".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}
