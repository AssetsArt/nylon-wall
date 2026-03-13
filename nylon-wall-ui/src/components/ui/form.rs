use crate::api_client;
use dioxus::prelude::*;

pub const INPUT_CLASS: &str = "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";
pub const SELECT_CLASS: &str = "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";

#[component]
pub fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "{label}" }
            {children}
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct InterfaceInfo {
    name: String,
}

/// A checkbox list for selecting multiple interfaces.
#[component]
pub fn MultiInterfaceSelect(
    value: String,
    onchange: EventHandler<String>,
    #[props(default)] label: Option<String>,
) -> Element {
    let interfaces = use_resource(|| async {
        api_client::get::<Vec<InterfaceInfo>>("/system/interfaces").await
    });

    let label_text = label.unwrap_or_else(|| "Interfaces".to_string());
    let selected: Vec<String> = value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    rsx! {
        FormField { label: label_text,
            match &*interfaces.read() {
                Some(Ok(list)) => rsx! {
                    div { class: "flex flex-wrap gap-2",
                        for iface in list.iter() {
                            {
                                let name = iface.name.clone();
                                let name2 = iface.name.clone();
                                let is_selected = selected.contains(&iface.name);
                                rsx! {
                                    button {
                                        key: "{name}",
                                        class: if is_selected {
                                            "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/20 text-blue-400 border border-blue-500/30 transition-colors"
                                        } else {
                                            "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-400 border border-slate-700/40 hover:bg-slate-700/50 transition-colors"
                                        },
                                        onclick: {
                                            let selected = selected.clone();
                                            move |_| {
                                                let mut new_selected = selected.clone();
                                                if new_selected.contains(&name2) {
                                                    new_selected.retain(|s| s != &name2);
                                                } else {
                                                    new_selected.push(name2.clone());
                                                }
                                                onchange.call(new_selected.join(", "));
                                            }
                                        },
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                },
                Some(Err(_)) => rsx! {
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "eth0, eth1", value: "{value}",
                        oninput: move |e| onchange.call(e.value()),
                    }
                },
                None => rsx! {
                    span { class: "text-xs text-slate-500", "Loading interfaces..." }
                },
            }
        }
    }
}

/// A select dropdown that fetches available network interfaces from the API.
#[component]
pub fn InterfaceSelect(
    value: String,
    onchange: EventHandler<String>,
    #[props(default)] label: Option<String>,
    #[props(default = false)] allow_empty: bool,
) -> Element {
    let interfaces = use_resource(|| async {
        api_client::get::<Vec<InterfaceInfo>>("/system/interfaces").await
    });

    let label_text = label.unwrap_or_else(|| "Interface".to_string());

    rsx! {
        FormField { label: label_text,
            select {
                class: SELECT_CLASS,
                value: "{value}",
                onchange: move |e| onchange.call(e.value()),
                if allow_empty {
                    option { value: "", "— None —" }
                }
                match &*interfaces.read() {
                    Some(Ok(list)) => rsx! {
                        for iface in list.iter() {
                            option {
                                key: "{iface.name}",
                                value: "{iface.name}",
                                "{iface.name}"
                            }
                        }
                    },
                    Some(Err(_)) => rsx! {
                        option { value: "{value}", "{value} (offline)" }
                    },
                    None => rsx! {
                        option { value: "{value}", "Loading..." }
                    },
                }
            }
        }
    }
}
