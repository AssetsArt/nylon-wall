use super::ui::*;
use super::{ConfirmModal, notify_change, use_change_guard, use_refresh_trigger};
use crate::api_client;
use crate::models::{WgPeer, WgPeerStatus, WgServer};
use crate::ws_client;
use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::*;
use dioxus_free_icons::Icon;

#[component]
pub fn Vpn() -> Element {
    let refresh = use_refresh_trigger();
    let ws = ws_client::use_ws_events();
    let mut guard = use_change_guard();

    let mut server = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.wireguard();
        api_client::get::<WgServer>("/vpn/server").await
    });

    let mut peers = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.wireguard();
        api_client::get::<Vec<WgPeer>>("/vpn/peers").await
    });

    let statuses = use_resource(move || async move {
        let _ = refresh();
        let _ = ws.wireguard();
        api_client::get::<Vec<WgPeerStatus>>("/vpn/status").await
    });

    let mut editing_server = use_signal(|| false);
    let mut editing_peer = use_signal(|| None::<(bool, WgPeer)>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut error_msg = use_signal(|| None::<String>);

    let server_data = server.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let peers_data = peers.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let status_list = statuses
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let status_map: std::collections::HashMap<String, WgPeerStatus> = status_list
        .into_iter()
        .map(|s| (s.public_key.clone(), s))
        .collect();

    let server_enabled = server_data.as_ref().map(|s| s.enabled).unwrap_or(false);
    let connected_count = statuses.read().as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|s| s.iter().filter(|st| st.last_handshake != "0" && st.last_handshake != "(none)").count())
        .unwrap_or(0);

    rsx! {
        div {
            PageHeader {
                title: "WireGuard VPN".to_string(),
                subtitle: "Manage VPN server and peer connections.".to_string(),
                Btn {
                    color: Color::Blue,
                    label: if editing_peer().is_some() {
                        "Cancel".to_string()
                    } else {
                        "+ Add Peer".to_string()
                    },
                    onclick: move |_| {
                        if editing_peer().is_some() {
                            editing_peer.set(None);
                        } else {
                            editing_peer.set(Some((
                                false,
                                WgPeer {
                                    id: 0,
                                    name: String::new(),
                                    public_key: String::new(),
                                    private_key: String::new(),
                                    preshared_key: String::new(),
                                    allowed_ips: String::new(),
                                    persistent_keepalive: 25,
                                    enabled: true,
                                },
                            )));
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

            // Stats
            div { class: "grid grid-cols-3 gap-4 mb-6",
                StatCard {
                    label: "Server".to_string(),
                    value: if server_enabled { "Active".to_string() } else { "Inactive".to_string() },
                    color: if server_enabled { Color::Emerald } else { Color::Slate },
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdShieldCheck } },
                }
                StatCard {
                    label: "Peers".to_string(),
                    value: peers_data.as_ref().map(|p| p.len().to_string()).unwrap_or("-".to_string()),
                    color: Color::Blue,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdUsers } },
                }
                StatCard {
                    label: "Connected".to_string(),
                    value: connected_count.to_string(),
                    color: Color::Emerald,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdWifi } },
                }
            }

            // Server config section
            SectionHeader {
                icon: rsx! { Icon { width: 14, height: 14, icon: LdServer, class: "text-violet-400" } },
                title: "Server Configuration".to_string(),
            }

            if editing_server() {
                if let Some(srv) = server_data.clone() {
                    WgServerForm {
                        server: srv,
                        on_saved: move |_| {
                            editing_server.set(false);
                            server.restart();
                            notify_change(&mut guard);
                        },
                        on_cancel: move |_| editing_server.set(false),
                        error_msg: error_msg,
                    }
                }
            } else {
                // Server info card
                div { class: "bg-slate-900/60 border border-slate-800/60 rounded-xl p-5 mb-6",
                    div { class: "flex items-center justify-between mb-4",
                        div { class: "flex items-center gap-3",
                            div { class: "w-8 h-8 rounded-lg bg-violet-500/10 flex items-center justify-center",
                                Icon { width: 16, height: 16, icon: LdServer, class: "text-violet-400" }
                            }
                            div {
                                p { class: "text-sm font-medium text-white",
                                    {server_data.as_ref().map(|s| s.interface.clone()).unwrap_or("wg0".to_string())}
                                }
                                p { class: "text-xs text-slate-500",
                                    {server_data.as_ref().map(|s| format!("{}:{}", s.address, s.listen_port)).unwrap_or("-".to_string())}
                                }
                            }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: if server_enabled {
                                    "px-3 py-1.5 text-xs font-medium rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20 transition-colors"
                                } else {
                                    "px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800 text-slate-400 border border-slate-700 hover:bg-slate-700 transition-colors"
                                },
                                onclick: move |_| {
                                    spawn(async move {
                                        match api_client::post::<(), WgServer>("/vpn/server/toggle", &()).await {
                                            Ok(_) => server.restart(),
                                            Err(e) => error_msg.set(Some(e)),
                                        }
                                    });
                                },
                                if server_enabled { "Enabled" } else { "Disabled" }
                            }
                            Btn {
                                color: Color::Slate,
                                label: "Edit".to_string(),
                                onclick: move |_| editing_server.set(true),
                            }
                        }
                    }
                    // Server details grid
                    if let Some(srv) = server_data.as_ref() {
                        div { class: "grid grid-cols-2 gap-x-8 gap-y-2 text-xs",
                            div { class: "text-slate-500", "Public Key" }
                            div { class: "text-slate-300 font-mono truncate",
                                {if srv.public_key.is_empty() {
                                    "Not generated".to_string()
                                } else {
                                    let short: String = srv.public_key.chars().take(20).collect();
                                    format!("{}...", short)
                                }}
                            }
                            div { class: "text-slate-500", "Endpoint" }
                            div { class: "text-slate-300",
                                {if srv.endpoint.is_empty() { "Not set".to_string() } else { format!("{}:{}", srv.endpoint, srv.listen_port) }}
                            }
                            div { class: "text-slate-500", "DNS" }
                            div { class: "text-slate-300",
                                {if srv.dns.is_empty() { "None".to_string() } else { srv.dns.join(", ") }}
                            }
                        }
                    }
                }
            }

            // Peer edit form
            if let Some((is_edit, peer)) = editing_peer() {
                WgPeerForm {
                    is_edit: is_edit,
                    peer: peer,
                    on_saved: move |_| {
                        editing_peer.set(None);
                        peers.restart();
                        notify_change(&mut guard);
                    },
                    on_cancel: move |_| editing_peer.set(None),
                    error_msg: error_msg,
                }
            }

            // Peers table
            SectionHeader {
                icon: rsx! { Icon { width: 14, height: 14, icon: LdUsers, class: "text-blue-400" } },
                title: "Peers".to_string(),
            }

            if let Some(peer_list) = peers_data.as_ref() {
                if peer_list.is_empty() {
                    EmptyState {
                        icon: rsx! { Icon { width: 48, height: 48, icon: LdUsers, class: "text-slate-700" } },
                        title: "No peers yet".to_string(),
                        subtitle: "Add a peer to create a VPN connection.".to_string(),
                    }
                } else {
                    div { class: "space-y-3 mb-6",
                        for peer in peer_list.iter() {
                            {
                                let peer_id = peer.id;
                                let peer_name = peer.name.clone();
                                let peer_ips = peer.allowed_ips.clone();
                                let peer_enabled = peer.enabled;
                                let peer_pubkey = peer.public_key.clone();

                                let st = status_map.get(&peer_pubkey);
                                let has_handshake = st
                                    .map(|s| s.last_handshake != "0" && s.last_handshake != "(none)")
                                    .unwrap_or(false);
                                let rx = st.map(|s| format_bytes(s.transfer_rx)).unwrap_or("-".to_string());
                                let tx = st.map(|s| format_bytes(s.transfer_tx)).unwrap_or("-".to_string());

                                rsx! {
                                    div { class: "bg-slate-900/60 border border-slate-800/60 rounded-xl p-4",
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-3",
                                                div {
                                                    class: if has_handshake && peer_enabled {
                                                        "w-2 h-2 rounded-full bg-emerald-400"
                                                    } else if peer_enabled {
                                                        "w-2 h-2 rounded-full bg-amber-400"
                                                    } else {
                                                        "w-2 h-2 rounded-full bg-slate-600"
                                                    },
                                                }
                                                div {
                                                    p { class: "text-sm font-medium text-white", {peer_name.clone()} }
                                                    p { class: "text-xs text-slate-500 font-mono", {peer_ips.clone()} }
                                                }
                                            }
                                            div { class: "flex items-center gap-4",
                                                // Transfer stats
                                                div { class: "text-xs text-slate-500 text-right",
                                                    span { class: "text-emerald-400", "↓ {rx}" }
                                                    " / "
                                                    span { class: "text-blue-400", "↑ {tx}" }
                                                }
                                                // Actions
                                                div { class: "flex items-center gap-1",
                                                    button {
                                                        class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                        title: "Download config",
                                                        onclick: move |_| {
                                                            let url = api_client::base_url(&format!("/vpn/peers/{}/config", peer_id));
                                                            let js = format!("window.open('{}', '_blank')", url);
                                                            dioxus::document::eval(&js);
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdDownload, class: "text-slate-400" }
                                                    }
                                                    button {
                                                        class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                        title: if peer_enabled { "Disable" } else { "Enable" },
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                let _ = api_client::post::<(), WgPeer>(
                                                                    &format!("/vpn/peers/{}/toggle", peer_id),
                                                                    &(),
                                                                ).await;
                                                                peers.restart();
                                                            });
                                                        },
                                                        if peer_enabled {
                                                            Icon { width: 14, height: 14, icon: LdToggleRight, class: "text-emerald-400" }
                                                        } else {
                                                            Icon { width: 14, height: 14, icon: LdToggleLeft, class: "text-slate-500" }
                                                        }
                                                    }
                                                    button {
                                                        class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                        title: "Edit",
                                                        onclick: {
                                                            let peer_clone = peer.clone();
                                                            move |_| {
                                                                editing_peer.set(Some((true, peer_clone.clone())));
                                                            }
                                                        },
                                                        Icon { width: 14, height: 14, icon: LdPencil, class: "text-slate-400" }
                                                    }
                                                    button {
                                                        class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-red-900/30 flex items-center justify-center transition-colors",
                                                        title: "Delete",
                                                        onclick: move |_| confirm_delete.set(Some(peer_id)),
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
            }

            // Delete confirmation
            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete Peer".to_string(),
                    message: format!("Are you sure you want to delete peer #{}?", del_id),
                    on_confirm: move |_| {
                        spawn(async move {
                            match api_client::delete(&format!("/vpn/peers/{}", del_id)).await {
                                Ok(()) => {
                                    confirm_delete.set(None);
                                    peers.restart();
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

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[component]
fn WgServerForm(
    server: WgServer,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    error_msg: Signal<Option<String>>,
) -> Element {
    let mut listen_port = use_signal(|| server.listen_port.to_string());
    let mut address = use_signal(|| server.address.clone());
    let mut endpoint = use_signal(|| server.endpoint.clone());
    let mut interface = use_signal(|| server.interface.clone());
    let mut dns = use_signal(|| server.dns.join(", "));
    let mut saving = use_signal(|| false);

    rsx! {
        FormCard {
            p { class: "text-sm font-medium text-white mb-4", "Server Settings" }
            div { class: "grid grid-cols-2 gap-4",
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Interface" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        value: "{interface}",
                        oninput: move |e| interface.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Listen Port" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        r#type: "number",
                        value: "{listen_port}",
                        oninput: move |e| listen_port.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Address (CIDR)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "10.0.0.1/24",
                        value: "{address}",
                        oninput: move |e| address.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Endpoint (public IP/hostname)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "vpn.example.com",
                        value: "{endpoint}",
                        oninput: move |e| endpoint.set(e.value()),
                    }
                }
                label { class: "col-span-2 block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "DNS Servers (comma-separated)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "1.1.1.1, 8.8.8.8",
                        value: "{dns}",
                        oninput: move |e| dns.set(e.value()),
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
                    label: "Save Server".to_string(),
                    disabled: saving(),
                    onclick: move |_| {
                        saving.set(true);
                        let srv = WgServer {
                            listen_port: listen_port().parse().unwrap_or(51820),
                            address: address(),
                            dns: dns().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                            private_key: server.private_key.clone(),
                            public_key: server.public_key.clone(),
                            interface: interface(),
                            enabled: server.enabled,
                            endpoint: endpoint(),
                        };
                        spawn(async move {
                            match api_client::put::<WgServer, WgServer>("/vpn/server", &srv).await {
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
fn WgPeerForm(
    is_edit: bool,
    peer: WgPeer,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    error_msg: Signal<Option<String>>,
) -> Element {
    let mut name = use_signal(|| peer.name.clone());
    let mut allowed_ips = use_signal(|| peer.allowed_ips.clone());
    let mut keepalive = use_signal(|| peer.persistent_keepalive.to_string());
    let mut saving = use_signal(|| false);

    rsx! {
        FormCard {
            p { class: "text-sm font-medium text-white mb-4",
                if is_edit { "Edit Peer" } else { "New Peer" }
            }
            div { class: "grid grid-cols-2 gap-4",
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Name" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "Phone, Laptop, etc.",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Allowed IPs" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        placeholder: "Auto-assigned if empty",
                        value: "{allowed_ips}",
                        oninput: move |e| allowed_ips.set(e.value()),
                    }
                }
                label { class: "block",
                    span { class: "text-xs font-medium text-slate-400 mb-1 block", "Persistent Keepalive (sec)" }
                    input {
                        class: "w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white",
                        r#type: "number",
                        value: "{keepalive}",
                        oninput: move |e| keepalive.set(e.value()),
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
                    label: if is_edit { "Update Peer".to_string() } else { "Create Peer".to_string() },
                    disabled: saving(),
                    onclick: move |_| {
                        saving.set(true);
                        let p = WgPeer {
                            id: peer.id,
                            name: name(),
                            public_key: peer.public_key.clone(),
                            private_key: peer.private_key.clone(),
                            preshared_key: peer.preshared_key.clone(),
                            allowed_ips: allowed_ips(),
                            persistent_keepalive: keepalive().parse().unwrap_or(25),
                            enabled: peer.enabled,
                        };
                        let is_edit = is_edit;
                        spawn(async move {
                            let result = if is_edit {
                                api_client::put::<WgPeer, WgPeer>(&format!("/vpn/peers/{}", p.id), &p).await
                            } else {
                                api_client::post::<WgPeer, WgPeer>("/vpn/peers", &p).await
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
