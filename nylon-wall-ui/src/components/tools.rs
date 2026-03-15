use super::ui::*;
use super::{ConfirmModal, notify_change, use_change_guard, use_refresh_trigger};
use crate::api_client;
use crate::models::{MdnsConfig, WolDevice, WolRequest};
use crate::ws_client;
use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;

#[component]
pub fn Tools() -> Element {
    rsx! {
        div {
            PageHeader {
                title: "Tools".to_string(),
                subtitle: "Network utilities and management tools.".to_string(),
            }
            WakeOnLan {}
            MdnsReflector {}
        }
    }
}

// === Wake-on-LAN ===

#[component]
fn WakeOnLan() -> Element {
    let refresh = use_refresh_trigger();
    let ws = ws_client::use_ws_events();
    let mut guard = use_change_guard();

    let mut devices = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.wol();
        api_client::get::<Vec<WolDevice>>("/tools/wol/devices").await
    });

    let mut show_form = use_signal(|| false);
    let mut editing = use_signal(|| None::<WolDevice>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut waking_id = use_signal(|| None::<u32>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut quick_mac = use_signal(String::new);

    rsx! {
        div { class: "mb-8",
            SectionHeader {
                icon: rsx! { Icon { width: 16, height: 16, icon: LdPower, class: "text-amber-400" } },
                title: "Wake-on-LAN".to_string(),
                Btn {
                    color: Color::Blue,
                    label: if show_form() { "Cancel".to_string() } else { "+ Add Device".to_string() },
                    onclick: move |_| {
                        if show_form() {
                            show_form.set(false);
                            editing.set(None);
                        } else {
                            editing.set(None);
                            show_form.set(true);
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

            // Quick wake by MAC
            div { class: "mb-4 flex items-center gap-2",
                input {
                    class: "flex-1 max-w-xs px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 font-mono",
                    r#type: "text",
                    placeholder: "aa:bb:cc:dd:ee:ff",
                    value: "{quick_mac}",
                    oninput: move |e| quick_mac.set(e.value()),
                }
                Btn {
                    color: Color::Amber,
                    label: "Wake".to_string(),
                    disabled: quick_mac().len() < 17,
                    onclick: move |_| {
                        let mac = quick_mac();
                        spawn(async move {
                            let body = WolRequest { mac: mac.clone(), interface: String::new() };
                            match api_client::post::<WolRequest, serde_json::Value>("/tools/wol", &body).await {
                                Ok(_) => {
                                    error_msg.set(None);
                                    quick_mac.set(String::new());
                                }
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    icon: rsx! { Icon { width: 12, height: 12, icon: LdZap } },
                }
            }

            // Add/Edit form
            if show_form() {
                WolForm {
                    device: editing(),
                    on_saved: move |_| {
                        show_form.set(false);
                        editing.set(None);
                        devices.restart();
                        notify_change(&mut guard);
                    },
                    on_cancel: move |_| {
                        show_form.set(false);
                        editing.set(None);
                    },
                    error_msg: error_msg,
                }
            }

            // Devices
            match &*devices.read() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    EmptyState {
                        icon: rsx! { Icon { width: 32, height: 32, icon: LdPower } },
                        title: "No Saved Devices".to_string(),
                        subtitle: "Save devices for quick Wake-on-LAN access.".to_string(),
                    }
                },
                Some(Ok(list)) => {
                    let mut sorted = list.clone();
                    sorted.sort_by_key(|d| d.id);
                    rsx! {
                        div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4",
                            for device in sorted {
                                {
                                    let dev_id = device.id;
                                    let dev_edit = device.clone();
                                    let is_waking = waking_id() == Some(device.id);
                                    rsx! {
                                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-4 hover:border-slate-700/60 transition-colors",
                                            div { class: "flex items-start justify-between mb-3",
                                                div {
                                                    h4 { class: "text-sm font-semibold text-white", "{device.name}" }
                                                    p { class: "text-xs font-mono text-slate-500 mt-0.5", "{device.mac}" }
                                                    if !device.interface.is_empty() {
                                                        p { class: "text-[10px] text-slate-600 mt-0.5", "Interface: {device.interface}" }
                                                    }
                                                }
                                                div { class: "flex items-center gap-1",
                                                    button {
                                                        class: "w-7 h-7 rounded-lg hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                        title: "Edit",
                                                        onclick: move |_| {
                                                            editing.set(Some(dev_edit.clone()));
                                                            show_form.set(true);
                                                        },
                                                        Icon { width: 12, height: 12, icon: LdPencil, class: "text-slate-400" }
                                                    }
                                                    button {
                                                        class: "w-7 h-7 rounded-lg hover:bg-red-500/10 flex items-center justify-center transition-colors",
                                                        title: "Delete",
                                                        onclick: move |_| confirm_delete.set(Some(dev_id)),
                                                        Icon { width: 12, height: 12, icon: LdTrash2, class: "text-red-400" }
                                                    }
                                                }
                                            }
                                            if let Some(ts) = &device.last_wake {
                                                p { class: "text-[10px] text-slate-600 mb-3",
                                                    "Last wake: {format_time(ts)}"
                                                }
                                            }
                                            button {
                                                class: "w-full px-3 py-2 rounded-lg text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20 hover:bg-amber-500/20 transition-colors disabled:opacity-50 flex items-center justify-center gap-1.5",
                                                disabled: is_waking,
                                                onclick: move |_| {
                                                    waking_id.set(Some(dev_id));
                                                    spawn(async move {
                                                        let _ = api_client::post::<(), serde_json::Value>(&format!("/tools/wol/devices/{}/wake", dev_id), &()).await;
                                                        waking_id.set(None);
                                                        devices.restart();
                                                    });
                                                },
                                                Icon { width: 12, height: 12, icon: LdZap }
                                                if is_waking { "Sending..." } else { "Wake Up" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! {},
            }

            // Delete confirmation
            if let Some(id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Device".to_string(),
                    message: "Remove this saved WOL device?".to_string(),
                    on_confirm: move |_| {
                        spawn(async move {
                            let _ = api_client::delete(&format!("/tools/wol/devices/{}", id)).await;
                            confirm_delete.set(None);
                            devices.restart();
                        });
                    },
                    on_cancel: move |_| confirm_delete.set(None),
                }
            }
        }
    }
}

// === mDNS Reflector ===

/// Simple interface info from /api/v1/system/interfaces
#[derive(serde::Deserialize, Clone)]
struct NetIface {
    name: String,
}

#[component]
fn MdnsReflector() -> Element {
    let ws = ws_client::use_ws_events();

    let mut config = use_resource(move || async move {
        let _ = ws.mdns();
        api_client::get::<MdnsConfig>("/tools/mdns").await
    });

    let interfaces = use_resource(|| async {
        api_client::get::<Vec<NetIface>>("/system/interfaces").await
    });

    let mut error_msg = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);

    rsx! {
        div { class: "mb-8",
            SectionHeader {
                icon: rsx! { Icon { width: 16, height: 16, icon: LdRadio, class: "text-violet-400" } },
                title: "mDNS Reflector".to_string(),
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            match (&*config.read(), &*interfaces.read()) {
                (Some(Ok(cfg)), Some(Ok(ifaces))) => {
                    let enabled = cfg.enabled;
                    let selected = cfg.interfaces.clone();
                    rsx! {
                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6",
                            div { class: "flex items-center justify-between mb-4",
                                div {
                                    h4 { class: "text-sm font-semibold text-white", "Reflect mDNS between interfaces" }
                                    p { class: "text-xs text-slate-500 mt-0.5",
                                        "Forward Bonjour/Avahi discovery packets across VLANs and subnets."
                                    }
                                }
                                div { class: "flex items-center gap-3",
                                    Badge {
                                        color: if enabled { Color::Emerald } else { Color::Slate },
                                        label: if enabled { "Active".to_string() } else { "Disabled".to_string() },
                                    }
                                    Btn {
                                        color: if enabled { Color::Slate } else { Color::Emerald },
                                        label: if enabled { "Disable".to_string() } else { "Enable".to_string() },
                                        onclick: move |_| {
                                            spawn(async move {
                                                match api_client::post::<(), MdnsConfig>("/tools/mdns/toggle", &()).await {
                                                    Ok(_) => config.restart(),
                                                    Err(e) => error_msg.set(Some(e)),
                                                }
                                            });
                                        },
                                    }
                                }
                            }

                            // Interface selection
                            p { class: "text-xs font-medium text-slate-400 mb-2", "Interfaces" }
                            div { class: "flex flex-wrap gap-2 mb-4",
                                for iface in ifaces {
                                    {
                                        let name = iface.name.clone();
                                        let is_selected = selected.contains(&name);
                                        let sel_clone = selected.clone();
                                        rsx! {
                                            button {
                                                class: if is_selected {
                                                    "px-3 py-1.5 rounded-lg text-xs font-medium bg-violet-500/20 text-violet-300 border border-violet-500/30 transition-colors"
                                                } else {
                                                    "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-500 border border-slate-700/40 hover:text-slate-300 hover:border-slate-600/40 transition-colors"
                                                },
                                                r#type: "button",
                                                onclick: move |_| {
                                                    let mut new_list = sel_clone.clone();
                                                    let n = name.clone();
                                                    if is_selected {
                                                        new_list.retain(|x| x != &n);
                                                    } else {
                                                        new_list.push(n);
                                                    }
                                                    let new_cfg = MdnsConfig {
                                                        enabled,
                                                        interfaces: new_list,
                                                    };
                                                    saving.set(true);
                                                    spawn(async move {
                                                        match api_client::put::<MdnsConfig, MdnsConfig>("/tools/mdns", &new_cfg).await {
                                                            Ok(_) => config.restart(),
                                                            Err(e) => error_msg.set(Some(e)),
                                                        }
                                                        saving.set(false);
                                                    });
                                                },
                                                "{iface.name}"
                                            }
                                        }
                                    }
                                }
                            }

                            if selected.len() < 2 && enabled {
                                p { class: "text-xs text-amber-400",
                                    "Select at least 2 interfaces to reflect mDNS packets between."
                                }
                            }

                            if !selected.is_empty() {
                                p { class: "text-[10px] text-slate-600",
                                    "Selected: {selected.join(\", \")}"
                                }
                            }
                        }
                    }
                },
                _ => rsx! {},
            }
        }
    }
}

fn format_time(ts: &str) -> String {
    if let Some(t) = ts.get(..19) {
        t.replace('T', " ")
    } else {
        ts.to_string()
    }
}

#[component]
fn WolForm(
    device: Option<WolDevice>,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    mut error_msg: Signal<Option<String>>,
) -> Element {
    let is_edit = device.is_some();
    let initial = device.unwrap_or(WolDevice {
        id: 0,
        name: String::new(),
        mac: String::new(),
        interface: String::new(),
        last_wake: None,
    });

    let entry_id = initial.id;
    let mut name = use_signal(move || initial.name.clone());
    let mut mac = use_signal(move || initial.mac.clone());
    let mut interface = use_signal(move || initial.interface.clone());
    let mut submitting = use_signal(|| false);

    rsx! {
        FormCard {
            form {
                onsubmit: move |e| {
                    e.prevent_default();
                    if submitting() { return; }
                    submitting.set(true);
                    error_msg.set(None);
                    spawn(async move {
                        let body = WolDevice {
                            id: entry_id,
                            name: name(),
                            mac: mac(),
                            interface: interface(),
                            last_wake: None,
                        };
                        let result = if is_edit {
                            api_client::put::<WolDevice, WolDevice>(&format!("/tools/wol/devices/{}", entry_id), &body).await
                        } else {
                            api_client::post::<WolDevice, WolDevice>("/tools/wol/devices", &body).await
                        };
                        match result {
                            Ok(_) => on_saved.call(()),
                            Err(e) => error_msg.set(Some(e)),
                        }
                        submitting.set(false);
                    });
                },
                class: "space-y-4",

                h3 { class: "text-sm font-semibold text-white mb-3",
                    if is_edit { "Edit Device" } else { "New Device" }
                }

                div { class: "grid grid-cols-3 gap-4",
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Device Name" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                            r#type: "text",
                            placeholder: "Desktop PC",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "MAC Address" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 font-mono",
                            r#type: "text",
                            placeholder: "aa:bb:cc:dd:ee:ff",
                            value: "{mac}",
                            oninput: move |e| mac.set(e.value()),
                        }
                    }
                    div {
                        label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Interface (optional)" }
                        input {
                            class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                            r#type: "text",
                            placeholder: "eth0",
                            value: "{interface}",
                            oninput: move |e| interface.set(e.value()),
                        }
                    }
                }

                div { class: "flex gap-2 pt-2",
                    SubmitBtn {
                        color: Color::Blue,
                        label: if submitting() { "Saving...".to_string() } else if is_edit { "Update".to_string() } else { "Save Device".to_string() },
                        disabled: submitting() || name().is_empty() || mac().is_empty(),
                        onclick: move |_| {},
                    }
                    Btn {
                        color: Color::Slate,
                        label: "Cancel".to_string(),
                        onclick: move |_| on_cancel.call(()),
                    }
                }
            }
        }
    }
}
