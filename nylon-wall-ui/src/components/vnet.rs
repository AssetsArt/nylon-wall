use super::ui::*;
use super::{ConfirmModal, notify_change, use_change_guard, use_refresh_trigger};
use crate::api_client;
use crate::models::{BridgeConfig, VlanConfig};
use crate::ws_client;
use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;

#[component]
pub fn Vnet() -> Element {
    let refresh = use_refresh_trigger();
    let ws = ws_client::use_ws_events();

    let vlans = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.vnet();
        api_client::get::<Vec<VlanConfig>>("/vnet/vlans").await
    });

    let bridges = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.vnet();
        api_client::get::<Vec<BridgeConfig>>("/vnet/bridges").await
    });

    let mut active_tab = use_signal(|| 0u8);

    let vlan_data = vlans.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let bridge_data = bridges.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();

    let vlan_count = vlan_data.as_ref().map(|v| v.len()).unwrap_or(0);
    let vlan_active = vlan_data.as_ref().map(|v| v.iter().filter(|x| x.enabled).count()).unwrap_or(0);
    let bridge_count = bridge_data.as_ref().map(|b| b.len()).unwrap_or(0);
    let bridge_active = bridge_data.as_ref().map(|b| b.iter().filter(|x| x.enabled).count()).unwrap_or(0);

    rsx! {
        div {
            PageHeader {
                title: "Virtual Networking".to_string(),
                subtitle: "Manage VLANs and Linux bridges.".to_string(),
            }

            // Stats
            div { class: "grid grid-cols-4 gap-4 mb-6",
                StatCard {
                    label: "VLANs".to_string(),
                    value: vlan_count.to_string(),
                    color: Color::Blue,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdNetwork } },
                }
                StatCard {
                    label: "VLANs Active".to_string(),
                    value: vlan_active.to_string(),
                    color: Color::Emerald,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdActivity } },
                }
                StatCard {
                    label: "Bridges".to_string(),
                    value: bridge_count.to_string(),
                    color: Color::Violet,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdGitMerge } },
                }
                StatCard {
                    label: "Bridges Active".to_string(),
                    value: bridge_active.to_string(),
                    color: Color::Emerald,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdActivity } },
                }
            }

            // Tab bar
            div { class: "flex gap-1 mb-6 bg-slate-900/60 border border-slate-800/60 rounded-xl p-1 w-fit",
                button {
                    class: if active_tab() == 0 {
                        "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium text-blue-400 bg-blue-500/10"
                    } else {
                        "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 transition-colors"
                    },
                    onclick: move |_| active_tab.set(0),
                    Icon { width: 14, height: 14, icon: LdNetwork }
                    "VLANs"
                }
                button {
                    class: if active_tab() == 1 {
                        "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium text-blue-400 bg-blue-500/10"
                    } else {
                        "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 transition-colors"
                    },
                    onclick: move |_| active_tab.set(1),
                    Icon { width: 14, height: 14, icon: LdGitMerge }
                    "Bridges"
                }
            }

            match active_tab() {
                0 => rsx! { VlansTab { vlans: vlans } },
                _ => rsx! { BridgesTab { bridges: bridges } },
            }
        }
    }
}

#[component]
fn VlansTab(mut vlans: Resource<Result<Vec<VlanConfig>, String>>) -> Element {
    let mut guard = use_change_guard();
    let mut editing = use_signal(|| None::<(bool, VlanConfig)>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut error_msg = use_signal(|| None::<String>);

    let vlan_data = vlans.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-4",
                SectionHeader {
                    icon: rsx! { Icon { width: 14, height: 14, icon: LdNetwork, class: "text-blue-400" } },
                    title: "VLAN Interfaces".to_string(),
                }
                Btn {
                    color: Color::Blue,
                    label: if editing().is_some() { "Cancel".to_string() } else { "+ Add VLAN".to_string() },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((false, VlanConfig::default())));
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

            if let Some((is_edit, vlan)) = editing() {
                VlanForm {
                    is_edit: is_edit,
                    vlan: vlan,
                    on_saved: move |_| {
                        editing.set(None);
                        vlans.restart();
                        notify_change(&mut guard);
                    },
                    on_cancel: move |_| editing.set(None),
                    error_msg: error_msg,
                }
            }

            if let Some(vlan_list) = vlan_data.as_ref() {
                if vlan_list.is_empty() {
                    EmptyState {
                        icon: rsx! { Icon { width: 48, height: 48, icon: LdNetwork, class: "text-slate-700" } },
                        title: "No VLANs".to_string(),
                        subtitle: "Create a VLAN sub-interface to segment your network.".to_string(),
                    }
                } else {
                    div { class: "space-y-3",
                        for vlan in vlan_list.iter() {
                            {
                                let vlan_id = vlan.id;
                                let iface = vlan.iface_name();
                                let ip = vlan.ip_address.clone().unwrap_or_default();
                                let enabled = vlan.enabled;
                                let vid = vlan.vlan_id;

                                rsx! {
                                    div { class: "bg-slate-900/60 border border-slate-800/60 rounded-xl p-4",
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-3",
                                                div {
                                                    class: if enabled { "w-2 h-2 rounded-full bg-emerald-400" } else { "w-2 h-2 rounded-full bg-slate-600" },
                                                }
                                                div {
                                                    p { class: "text-sm font-medium text-white font-mono", {iface.clone()} }
                                                    div { class: "flex items-center gap-2 mt-0.5",
                                                        span { class: "text-xs text-slate-500", "VLAN {vid}" }
                                                        if !ip.is_empty() {
                                                            span { class: "text-xs text-slate-400 font-mono", {ip} }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "flex items-center gap-1",
                                                button {
                                                    class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                    title: if enabled { "Disable" } else { "Enable" },
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            let _ = api_client::post::<(), VlanConfig>(
                                                                &format!("/vnet/vlans/{}/toggle", vlan_id), &()
                                                            ).await;
                                                            vlans.restart();
                                                        });
                                                    },
                                                    if enabled {
                                                        Icon { width: 14, height: 14, icon: LdToggleRight, class: "text-emerald-400" }
                                                    } else {
                                                        Icon { width: 14, height: 14, icon: LdToggleLeft, class: "text-slate-500" }
                                                    }
                                                }
                                                button {
                                                    class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                    title: "Edit",
                                                    onclick: {
                                                        let vlan_clone = vlan.clone();
                                                        move |_| editing.set(Some((true, vlan_clone.clone())))
                                                    },
                                                    Icon { width: 14, height: 14, icon: LdPencil, class: "text-slate-400" }
                                                }
                                                button {
                                                    class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-red-900/30 flex items-center justify-center transition-colors",
                                                    title: "Delete",
                                                    onclick: move |_| confirm_delete.set(Some(vlan_id)),
                                                    Icon { width: 14, height: 14, icon: LdTrash2, class: "text-slate-400 hover:text-red-400" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete VLAN".to_string(),
                    message: format!("Are you sure you want to delete VLAN #{}?", del_id),
                    on_confirm: move |_| {
                        spawn(async move {
                            match api_client::delete(&format!("/vnet/vlans/{}", del_id)).await {
                                Ok(()) => {
                                    confirm_delete.set(None);
                                    vlans.restart();
                                    notify_change(&mut guard);
                                }
                                Err(e) => {
                                    confirm_delete.set(None);
                                    error_msg.set(Some(e));
                                }
                            }
                        });
                    },
                    on_cancel: move |_| confirm_delete.set(None),
                }
            }
        }
    }
}

#[component]
fn VlanForm(
    is_edit: bool,
    vlan: VlanConfig,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    error_msg: Signal<Option<String>>,
) -> Element {
    let mut parent = use_signal(|| vlan.parent_interface.clone());
    let mut vid = use_signal(|| vlan.vlan_id.to_string());
    let mut ip_addr = use_signal(|| vlan.ip_address.clone().unwrap_or_default());
    let mut saving = use_signal(|| false);

    rsx! {
        FormCard {
            p { class: "text-sm font-medium text-white mb-4",
                if is_edit { "Edit VLAN" } else { "New VLAN" }
            }
            div { class: "grid grid-cols-3 gap-4",
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Parent Interface" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "eth0",
                        value: "{parent}",
                        oninput: move |e| parent.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "VLAN ID (1-4094)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        r#type: "number",
                        min: "1",
                        max: "4094",
                        value: "{vid}",
                        oninput: move |e| vid.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "IP Address (CIDR, optional)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "192.168.10.1/24",
                        value: "{ip_addr}",
                        oninput: move |e| ip_addr.set(e.value()),
                    }
                }
            }
            div { class: "flex justify-end gap-2 mt-4",
                Btn {
                    color: Color::Slate,
                    label: "Cancel".to_string(),
                    onclick: move |_| on_cancel.call(()),
                }
                SubmitBtn {
                    color: Color::Blue,
                    label: if is_edit { "Update".to_string() } else { "Create".to_string() },
                    disabled: saving(),
                    onclick: move |_| {
                        saving.set(true);
                        let v = VlanConfig {
                            id: vlan.id,
                            parent_interface: parent(),
                            vlan_id: vid().parse().unwrap_or(10),
                            ip_address: if ip_addr().is_empty() { None } else { Some(ip_addr()) },
                            enabled: vlan.enabled,
                        };
                        let is_edit = is_edit;
                        spawn(async move {
                            let result = if is_edit {
                                api_client::put::<VlanConfig, VlanConfig>(&format!("/vnet/vlans/{}", v.id), &v).await
                            } else {
                                api_client::post::<VlanConfig, VlanConfig>("/vnet/vlans", &v).await
                            };
                            match result {
                                Ok(_) => on_saved.call(()),
                                Err(e) => error_msg.set(Some(e)),
                            }
                            saving.set(false);
                        });
                    },
                }
            }
        }
    }
}

#[component]
fn BridgesTab(mut bridges: Resource<Result<Vec<BridgeConfig>, String>>) -> Element {
    let mut guard = use_change_guard();
    let mut editing = use_signal(|| None::<(bool, BridgeConfig)>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut error_msg = use_signal(|| None::<String>);

    let bridge_data = bridges.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-4",
                SectionHeader {
                    icon: rsx! { Icon { width: 14, height: 14, icon: LdGitMerge, class: "text-violet-400" } },
                    title: "Linux Bridges".to_string(),
                }
                Btn {
                    color: Color::Blue,
                    label: if editing().is_some() { "Cancel".to_string() } else { "+ Add Bridge".to_string() },
                    onclick: move |_| {
                        if editing().is_some() {
                            editing.set(None);
                        } else {
                            editing.set(Some((false, BridgeConfig::default())));
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

            if let Some((is_edit, bridge)) = editing() {
                BridgeForm {
                    is_edit: is_edit,
                    bridge: bridge,
                    on_saved: move |_| {
                        editing.set(None);
                        bridges.restart();
                        notify_change(&mut guard);
                    },
                    on_cancel: move |_| editing.set(None),
                    error_msg: error_msg,
                }
            }

            if let Some(bridge_list) = bridge_data.as_ref() {
                if bridge_list.is_empty() {
                    EmptyState {
                        icon: rsx! { Icon { width: 48, height: 48, icon: LdGitMerge, class: "text-slate-700" } },
                        title: "No Bridges".to_string(),
                        subtitle: "Create a Linux bridge to connect network segments.".to_string(),
                    }
                } else {
                    div { class: "space-y-3",
                        for bridge in bridge_list.iter() {
                            {
                                let br_id = bridge.id;
                                let br_name = bridge.name.clone();
                                let br_ip = bridge.ip_address.clone().unwrap_or_default();
                                let br_enabled = bridge.enabled;
                                let br_stp = bridge.stp_enabled;
                                let port_list = bridge.ports.join(", ");

                                rsx! {
                                    div { class: "bg-slate-900/60 border border-slate-800/60 rounded-xl p-4",
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-3",
                                                div {
                                                    class: if br_enabled { "w-2 h-2 rounded-full bg-emerald-400" } else { "w-2 h-2 rounded-full bg-slate-600" },
                                                }
                                                div {
                                                    div { class: "flex items-center gap-2",
                                                        p { class: "text-sm font-medium text-white font-mono", {br_name.clone()} }
                                                        if br_stp {
                                                            Badge { color: Color::Amber, label: "STP".to_string() }
                                                        }
                                                    }
                                                    div { class: "flex items-center gap-2 mt-0.5",
                                                        if !br_ip.is_empty() {
                                                            span { class: "text-xs text-slate-400 font-mono", {br_ip} }
                                                        }
                                                        if !port_list.is_empty() {
                                                            span { class: "text-xs text-slate-500", "Ports: {port_list}" }
                                                        } else {
                                                            span { class: "text-xs text-slate-600", "No ports" }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "flex items-center gap-1",
                                                button {
                                                    class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                    title: if br_enabled { "Disable" } else { "Enable" },
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            let _ = api_client::post::<(), BridgeConfig>(
                                                                &format!("/vnet/bridges/{}/toggle", br_id), &()
                                                            ).await;
                                                            bridges.restart();
                                                        });
                                                    },
                                                    if br_enabled {
                                                        Icon { width: 14, height: 14, icon: LdToggleRight, class: "text-emerald-400" }
                                                    } else {
                                                        Icon { width: 14, height: 14, icon: LdToggleLeft, class: "text-slate-500" }
                                                    }
                                                }
                                                button {
                                                    class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                    title: "Edit",
                                                    onclick: {
                                                        let br_clone = bridge.clone();
                                                        move |_| editing.set(Some((true, br_clone.clone())))
                                                    },
                                                    Icon { width: 14, height: 14, icon: LdPencil, class: "text-slate-400" }
                                                }
                                                button {
                                                    class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-red-900/30 flex items-center justify-center transition-colors",
                                                    title: "Delete",
                                                    onclick: move |_| confirm_delete.set(Some(br_id)),
                                                    Icon { width: 14, height: 14, icon: LdTrash2, class: "text-slate-400 hover:text-red-400" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Bridge".to_string(),
                    message: format!("Are you sure you want to delete bridge #{}?", del_id),
                    on_confirm: move |_| {
                        spawn(async move {
                            match api_client::delete(&format!("/vnet/bridges/{}", del_id)).await {
                                Ok(()) => {
                                    confirm_delete.set(None);
                                    bridges.restart();
                                    notify_change(&mut guard);
                                }
                                Err(e) => {
                                    confirm_delete.set(None);
                                    error_msg.set(Some(e));
                                }
                            }
                        });
                    },
                    on_cancel: move |_| confirm_delete.set(None),
                }
            }
        }
    }
}

#[component]
fn BridgeForm(
    is_edit: bool,
    bridge: BridgeConfig,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    error_msg: Signal<Option<String>>,
) -> Element {
    let mut name = use_signal(|| bridge.name.clone());
    let mut ports = use_signal(|| bridge.ports.join(", "));
    let mut ip_addr = use_signal(|| bridge.ip_address.clone().unwrap_or_default());
    let mut stp = use_signal(|| bridge.stp_enabled);
    let mut saving = use_signal(|| false);

    rsx! {
        FormCard {
            p { class: "text-sm font-medium text-white mb-4",
                if is_edit { "Edit Bridge" } else { "New Bridge" }
            }
            div { class: "grid grid-cols-2 gap-4",
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Bridge Name" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "br-lan",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "IP Address (CIDR, optional)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "192.168.1.1/24",
                        value: "{ip_addr}",
                        oninput: move |e| ip_addr.set(e.value()),
                    }
                }
                label { class: "col-span-2 block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Ports (comma-separated interfaces)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "eth1, eth2, eth0.100",
                        value: "{ports}",
                        oninput: move |e| ports.set(e.value()),
                    }
                }
                label { class: "flex items-center gap-2 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded bg-slate-800 border-slate-600",
                        checked: stp(),
                        oninput: move |e| stp.set(e.checked()),
                    }
                    span { class: "text-xs font-medium text-slate-400", "Enable STP (Spanning Tree Protocol)" }
                }
            }
            div { class: "flex justify-end gap-2 mt-4",
                Btn {
                    color: Color::Slate,
                    label: "Cancel".to_string(),
                    onclick: move |_| on_cancel.call(()),
                }
                SubmitBtn {
                    color: Color::Blue,
                    label: if is_edit { "Update".to_string() } else { "Create".to_string() },
                    disabled: saving(),
                    onclick: move |_| {
                        saving.set(true);
                        let b = BridgeConfig {
                            id: bridge.id,
                            name: name(),
                            ports: ports().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                            ip_address: if ip_addr().is_empty() { None } else { Some(ip_addr()) },
                            stp_enabled: stp(),
                            enabled: bridge.enabled,
                        };
                        let is_edit = is_edit;
                        spawn(async move {
                            let result = if is_edit {
                                api_client::put::<BridgeConfig, BridgeConfig>(&format!("/vnet/bridges/{}", b.id), &b).await
                            } else {
                                api_client::post::<BridgeConfig, BridgeConfig>("/vnet/bridges", &b).await
                            };
                            match result {
                                Ok(_) => on_saved.call(()),
                                Err(e) => error_msg.set(Some(e)),
                            }
                            saving.set(false);
                        });
                    },
                }
            }
        }
    }
}
