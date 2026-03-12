use dioxus::prelude::*;
use crate::api_client;
use crate::models::*;

#[component]
pub fn Nat() -> Element {
    let mut entries = use_resource(|| async {
        api_client::get::<Vec<NatEntry>>("/nat").await
    });
    let mut show_form = use_signal(|| false);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                h1 { "NAT Configuration" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ New NAT Entry" }
                }
            }

            if show_form() {
                NatForm {
                    on_saved: move |_| {
                        show_form.set(false);
                        entries.restart();
                    }
                }
            }

            match &*entries.read() {
                Some(Ok(list)) => rsx! {
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "ID" }
                                th { "Type" }
                                th { "Source" }
                                th { "Destination" }
                                th { "Translate To" }
                                th { "Status" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            for entry in list.iter() {
                                tr { key: "{entry.id}",
                                    td { "{entry.id}" }
                                    td {
                                        span { class: "badge",
                                            match entry.nat_type {
                                                NatType::SNAT => "SNAT",
                                                NatType::DNAT => "DNAT",
                                                NatType::Masquerade => "Masquerade",
                                            }
                                        }
                                    }
                                    td { {entry.src_network.clone().unwrap_or("Any".to_string())} }
                                    td { {entry.dst_network.clone().unwrap_or("Any".to_string())} }
                                    td { {entry.translate_ip.clone().unwrap_or("\u{2014}".to_string())} }
                                    td {
                                        span {
                                            class: if entry.enabled { "badge badge-success" } else { "badge badge-muted" },
                                            if entry.enabled { "Enabled" } else { "Disabled" }
                                        }
                                    }
                                    td {
                                        {
                                            let id = entry.id;
                                            rsx! {
                                                button {
                                                    class: "btn btn-sm btn-danger",
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            let _ = api_client::delete(&format!("/nat/{}", id)).await;
                                                            entries.restart();
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
                        p { class: "empty-state", "No NAT entries configured." }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "error", "Failed to load NAT entries: {e}" } },
                None => rsx! { p { "Loading..." } },
            }
        }
    }
}

#[component]
fn NatForm(on_saved: EventHandler<()>) -> Element {
    let mut nat_type = use_signal(|| "SNAT".to_string());
    let mut src_network = use_signal(|| String::new());
    let mut dst_network = use_signal(|| String::new());
    let mut translate_ip = use_signal(|| String::new());
    let mut out_interface = use_signal(|| String::new());

    let on_submit = move |_| {
        let entry = NatEntry {
            id: 0,
            nat_type: match nat_type().as_str() {
                "DNAT" => NatType::DNAT,
                "Masquerade" => NatType::Masquerade,
                _ => NatType::SNAT,
            },
            enabled: true,
            src_network: if src_network().is_empty() { None } else { Some(src_network()) },
            dst_network: if dst_network().is_empty() { None } else { Some(dst_network()) },
            protocol: None,
            dst_port: None,
            in_interface: None,
            out_interface: if out_interface().is_empty() { None } else { Some(out_interface()) },
            translate_ip: if translate_ip().is_empty() { None } else { Some(translate_ip()) },
            translate_port: None,
        };
        spawn(async move {
            match api_client::post::<NatEntry, NatEntry>("/nat", &entry).await {
                Ok(_) => on_saved.call(()),
                Err(e) => tracing::error!("Failed to create NAT entry: {}", e),
            }
        });
    };

    rsx! {
        div { class: "card form-card",
            h3 { "Create NAT Entry" }
            div { class: "form-grid",
                div { class: "form-group",
                    label { "NAT Type" }
                    select {
                        value: "{nat_type}",
                        onchange: move |e| nat_type.set(e.value()),
                        option { value: "SNAT", "SNAT" }
                        option { value: "DNAT", "DNAT" }
                        option { value: "Masquerade", "Masquerade" }
                    }
                }
                div { class: "form-group",
                    label { "Source Network" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 192.168.1.0/24",
                        value: "{src_network}",
                        oninput: move |e| src_network.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Destination Network" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 0.0.0.0/0",
                        value: "{dst_network}",
                        oninput: move |e| dst_network.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Translate IP" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. 203.0.113.1",
                        value: "{translate_ip}",
                        oninput: move |e| translate_ip.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Out Interface" }
                    input {
                        r#type: "text",
                        placeholder: "e.g. eth0",
                        value: "{out_interface}",
                        oninput: move |e| out_interface.set(e.value()),
                    }
                }
            }
            div { class: "form-actions",
                button { class: "btn btn-primary", onclick: on_submit, "Create Entry" }
            }
        }
    }
}
