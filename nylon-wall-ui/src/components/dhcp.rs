use super::ConfirmModal;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn Dhcp() -> Element {
    let mut active_tab = use_signal(|| 0u8); // 0=Pools, 1=Leases, 2=Client

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "DHCP" }
                    p { class: "text-sm text-slate-400 mt-1", "DHCP server pools, leases, and WAN client configuration" }
                }
            }

            // Tab navigation
            div { class: "flex gap-1 mb-6 bg-slate-900/50 rounded-xl p-1 border border-slate-800/40 w-fit",
                button {
                    class: if active_tab() == 0 {
                        "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 ring-1 ring-blue-500/20"
                    } else {
                        "px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
                    },
                    onclick: move |_| active_tab.set(0),
                    "Server Pools"
                }
                button {
                    class: if active_tab() == 1 {
                        "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 ring-1 ring-blue-500/20"
                    } else {
                        "px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
                    },
                    onclick: move |_| active_tab.set(1),
                    "Leases"
                }
                button {
                    class: if active_tab() == 2 {
                        "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 ring-1 ring-blue-500/20"
                    } else {
                        "px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
                    },
                    onclick: move |_| active_tab.set(2),
                    "WAN Client"
                }
            }

            match active_tab() {
                0 => rsx! { DhcpPoolsTab {} },
                1 => rsx! { DhcpLeasesTab {} },
                2 => rsx! { DhcpClientTab {} },
                _ => rsx! {},
            }
        }
    }
}

// === Tab 1: Server Pools ===

#[component]
fn DhcpPoolsTab() -> Element {
    let mut pools =
        use_resource(|| async { api_client::get::<Vec<DhcpPool>>("/dhcp/pools").await });
    let mut show_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<u32>);

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-lg font-medium text-white", "DHCP Server Pools" }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() { "Cancel" } else { "+ New Pool" }
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
                DhcpPoolForm {
                    on_saved: move |_| {
                        show_form.set(false);
                        pools.restart();
                    }
                }
            }

            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete DHCP Pool".to_string(),
                    message: format!("Are you sure you want to delete DHCP pool #{}? This will stop serving DHCP on that interface.", del_id),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/dhcp/pools/{}", del_id)).await {
                                Ok(_) => pools.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_delete.set(None); },
                }
            }

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Interface" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Subnet" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Range" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Gateway" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "DNS" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Lease" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*pools.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for pool in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{pool.id}",
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono", "{pool.interface}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono", "{pool.subnet}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono text-xs",
                                            "{pool.range_start} - {pool.range_end}"
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono",
                                            {pool.gateway.clone().unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 text-xs",
                                            {pool.dns_servers.join(", ")}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400",
                                            {format_lease_time(pool.lease_time)}
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let pool_id = pool.id;
                                                let is_enabled = pool.enabled;
                                                rsx! {
                                                    button {
                                                        class: if is_enabled {
                                                            "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 cursor-pointer hover:bg-emerald-500/20"
                                                        } else {
                                                            "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20 cursor-pointer hover:bg-slate-500/20"
                                                        },
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::post::<(), serde_json::Value>(&format!("/dhcp/pools/{}/toggle", pool_id), &()).await {
                                                                    Ok(_) => pools.restart(),
                                                                    Err(e) => error_msg.set(Some(e)),
                                                                }
                                                            });
                                                        },
                                                        if is_enabled { "Enabled" } else { "Disabled" }
                                                    }
                                                }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let id = pool.id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| confirm_delete.set(Some(id)),
                                                        Icon { width: 13, height: 13, icon: LdTrash2 }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "No DHCP pools configured" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "8", "Failed to load pools: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "Loading..." } }
                            },
                        }
                    }
                }
            }
        }
    }
}

// === Tab 2: Leases ===

#[component]
fn DhcpLeasesTab() -> Element {
    let mut leases =
        use_resource(|| async { api_client::get::<Vec<DhcpLease>>("/dhcp/leases").await });
    let mut reservations = use_resource(|| async {
        api_client::get::<Vec<DhcpReservation>>("/dhcp/reservations").await
    });
    let mut show_reservation_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_release = use_signal(|| None::<String>); // MAC to release

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-lg font-medium text-white", "Active Leases" }
                div { class: "flex items-center gap-2",
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-400 border border-slate-700/40 hover:bg-slate-700/50 transition-colors",
                        onclick: move |_| { leases.restart(); reservations.restart(); },
                        Icon { width: 12, height: 12, icon: LdRefreshCw }
                        span { class: "ml-1.5", "Refresh" }
                    }
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

            if let Some(ref mac) = confirm_release() {
                ConfirmModal {
                    title: "Release DHCP Lease".to_string(),
                    message: format!("Release the DHCP lease for MAC address {}?", mac),
                    confirm_label: "Release".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        let mac_val = confirm_release().unwrap();
                        confirm_release.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/dhcp/leases/{}", mac_val)).await {
                                Ok(_) => leases.restart(),
                                Err(e) => error_msg.set(Some(e)),
                            }
                        });
                    },
                    on_cancel: move |_| { confirm_release.set(None); },
                }
            }

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden mb-8",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "IP Address" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "MAC Address" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Hostname" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Expires" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "State" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*leases.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for lease in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{lease.mac}",
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono", "{lease.ip}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono text-xs", "{lease.mac}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400",
                                            {lease.hostname.clone().unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 text-xs",
                                            {format_timestamp(lease.lease_end)}
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: match lease.state {
                                                    DhcpLeaseState::Active => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
                                                    DhcpLeaseState::Reserved => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20",
                                                    DhcpLeaseState::Expired => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20",
                                                },
                                                match lease.state {
                                                    DhcpLeaseState::Active => "Active",
                                                    DhcpLeaseState::Reserved => "Reserved",
                                                    DhcpLeaseState::Expired => "Expired",
                                                }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let mac = lease.mac.clone();
                                                let mac2 = lease.mac.clone();
                                                rsx! {
                                                    div { class: "flex items-center gap-1",
                                                        button {
                                                            class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-blue-400 hover:bg-blue-500/10 transition-colors",
                                                            title: "Add reservation",
                                                            onclick: move |_| {
                                                                let mac_val = mac.clone();
                                                                spawn(async move {
                                                                    match api_client::post::<(), serde_json::Value>(&format!("/dhcp/leases/{}/reserve", mac_val), &()).await {
                                                                        Ok(_) => { leases.restart(); reservations.restart(); },
                                                                        Err(e) => error_msg.set(Some(e)),
                                                                    }
                                                                });
                                                            },
                                                            Icon { width: 13, height: 13, icon: LdBookmark }
                                                        }
                                                        button {
                                                            class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                            title: "Release lease",
                                                            onclick: move |_| confirm_release.set(Some(mac2.clone())),
                                                            Icon { width: 13, height: 13, icon: LdTrash2 }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Ok(_)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "No active DHCP leases" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "6", "Failed to load leases: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "6", "Loading..." } }
                            },
                        }
                    }
                }
            }

            // Reservations section
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-lg font-medium text-white", "Static Reservations" }
                button {
                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                    onclick: move |_| show_reservation_form.set(!show_reservation_form()),
                    if show_reservation_form() { "Cancel" } else { "+ New Reservation" }
                }
            }

            if show_reservation_form() {
                DhcpReservationForm {
                    on_saved: move |_| {
                        show_reservation_form.set(false);
                        reservations.restart();
                    }
                }
            }

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "MAC Address" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "IP Address" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Hostname" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Pool" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*reservations.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for res in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{res.id}",
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono text-xs", "{res.mac}" }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono", "{res.ip}" }
                                        td { class: "px-5 py-3 text-sm text-slate-400",
                                            {res.hostname.clone().unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400", "#{res.pool_id}" }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let id = res.id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/dhcp/reservations/{}", id)).await {
                                                                    Ok(_) => reservations.restart(),
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
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "5", "No static reservations" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "5", "Failed to load reservations: {e}" } }
                            },
                            None => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "5", "Loading..." } }
                            },
                        }
                    }
                }
            }
        }
    }
}

// === Tab 3: WAN Client ===

#[component]
fn DhcpClientTab() -> Element {
    let mut clients =
        use_resource(|| async { api_client::get::<Vec<DhcpClientConfig>>("/dhcp/clients").await });
    let mut statuses = use_resource(|| async {
        api_client::get::<Vec<DhcpClientStatus>>("/dhcp/clients/status").await
    });
    let mut show_form = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-lg font-medium text-white", "WAN DHCP Client" }
                div { class: "flex items-center gap-2",
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-400 border border-slate-700/40 hover:bg-slate-700/50 transition-colors",
                        onclick: move |_| { clients.restart(); statuses.restart(); },
                        Icon { width: 12, height: 12, icon: LdRefreshCw }
                        span { class: "ml-1.5", "Refresh" }
                    }
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                        onclick: move |_| show_form.set(!show_form()),
                        if show_form() { "Cancel" } else { "+ Add WAN Client" }
                    }
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
                DhcpClientForm {
                    on_saved: move |_| {
                        show_form.set(false);
                        clients.restart();
                        statuses.restart();
                    }
                }
            }

            // Client cards
            match &*clients.read() {
                Some(Ok(client_list)) if !client_list.is_empty() => {
                    let status_list = match &*statuses.read() {
                        Some(Ok(s)) => s.clone(),
                        _ => Vec::new(),
                    };
                    rsx! {
                        div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
                            for config in client_list.iter() {
                                {
                                    let status = status_list.iter().find(|s| s.interface == config.interface).cloned();
                                    let iface = config.interface.clone();
                                    let iface2 = config.interface.clone();
                                    let config_id = config.id;
                                    let is_enabled = config.enabled;
                                    rsx! {
                                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5",
                                            key: "{config.id}",
                                            div { class: "flex items-center justify-between mb-3",
                                                div { class: "flex items-center gap-2",
                                                    Icon { width: 16, height: 16, icon: LdWifi, class: "text-blue-400" }
                                                    span { class: "text-sm font-semibold text-white", "{config.interface}" }
                                                }
                                                div { class: "flex items-center gap-2",
                                                    {
                                                        let state_badge = if let Some(ref s) = status {
                                                            match s.state {
                                                                DhcpClientState::Bound | DhcpClientState::Renewing => ("bg-emerald-500/10 text-emerald-400 border-emerald-500/20", format!("{:?}", s.state)),
                                                                DhcpClientState::Discovering | DhcpClientState::Requesting | DhcpClientState::Rebinding => ("bg-amber-500/10 text-amber-400 border-amber-500/20", format!("{:?}", s.state)),
                                                                DhcpClientState::Error => ("bg-red-500/10 text-red-400 border-red-500/20", "Error".to_string()),
                                                                _ => ("bg-slate-500/10 text-slate-400 border-slate-500/20", "Idle".to_string()),
                                                            }
                                                        } else {
                                                            ("bg-slate-500/10 text-slate-400 border-slate-500/20", "Unknown".to_string())
                                                        };
                                                        rsx! {
                                                            span {
                                                                class: format!("px-2 py-0.5 rounded-full text-[11px] font-medium border {}", state_badge.0),
                                                                "{state_badge.1}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // Status details
                                            if let Some(ref s) = status {
                                                div { class: "grid grid-cols-2 gap-2 mb-4 text-xs",
                                                    div { class: "text-slate-500", "IP Address" }
                                                    div { class: "text-slate-300 font-mono",
                                                        {s.ip.clone().unwrap_or("\u{2014}".to_string())}
                                                    }
                                                    div { class: "text-slate-500", "Subnet Mask" }
                                                    div { class: "text-slate-300 font-mono",
                                                        {s.subnet_mask.clone().unwrap_or("\u{2014}".to_string())}
                                                    }
                                                    div { class: "text-slate-500", "Gateway" }
                                                    div { class: "text-slate-300 font-mono",
                                                        {s.gateway.clone().unwrap_or("\u{2014}".to_string())}
                                                    }
                                                    div { class: "text-slate-500", "DNS" }
                                                    div { class: "text-slate-300 font-mono text-xs",
                                                        {s.dns_servers.join(", ")}
                                                    }
                                                    div { class: "text-slate-500", "DHCP Server" }
                                                    div { class: "text-slate-300 font-mono",
                                                        {s.dhcp_server.clone().unwrap_or("\u{2014}".to_string())}
                                                    }
                                                }
                                            }

                                            // Actions
                                            div { class: "flex items-center gap-2 pt-3 border-t border-slate-800/40",
                                                button {
                                                    class: if is_enabled {
                                                        "px-3 py-1.5 rounded-lg text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20 transition-colors"
                                                    } else {
                                                        "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20 hover:bg-slate-500/20 transition-colors"
                                                    },
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            match api_client::post::<(), serde_json::Value>(&format!("/dhcp/clients/{}/toggle", config_id), &()).await {
                                                                Ok(_) => { clients.restart(); statuses.restart(); },
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                    if is_enabled { "Disable" } else { "Enable" }
                                                }
                                                button {
                                                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                                                    onclick: move |_| {
                                                        let iface_val = iface.clone();
                                                        spawn(async move {
                                                            match api_client::post::<(), serde_json::Value>(&format!("/dhcp/clients/{}/renew", iface_val), &()).await {
                                                                Ok(_) => statuses.restart(),
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                    "Renew"
                                                }
                                                button {
                                                    class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20 hover:bg-amber-500/20 transition-colors",
                                                    onclick: move |_| {
                                                        let iface_val = iface2.clone();
                                                        spawn(async move {
                                                            match api_client::post::<(), serde_json::Value>(&format!("/dhcp/clients/{}/release", iface_val), &()).await {
                                                                Ok(_) => statuses.restart(),
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                    "Release"
                                                }
                                                button {
                                                    class: "px-3 py-1.5 rounded-lg text-xs font-medium text-red-400 hover:bg-red-500/10 transition-colors ml-auto",
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            match api_client::delete(&format!("/dhcp/clients/{}", config_id)).await {
                                                                Ok(_) => { clients.restart(); statuses.restart(); },
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
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    div { class: "text-center py-16 text-sm text-slate-600",
                        "No WAN DHCP clients configured"
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "text-center py-16 text-sm text-red-400",
                        "Failed to load DHCP clients: {e}"
                    }
                },
                None => rsx! {
                    div { class: "text-center py-16 text-sm text-slate-600",
                        "Loading..."
                    }
                },
            }
        }
    }
}

// === Forms ===

#[component]
fn DhcpPoolForm(on_saved: EventHandler<()>) -> Element {
    let mut interface = use_signal(|| "eth1".to_string());
    let mut subnet = use_signal(String::new);
    let mut range_start = use_signal(String::new);
    let mut range_end = use_signal(String::new);
    let mut gateway = use_signal(String::new);
    let mut dns_servers = use_signal(|| "8.8.8.8, 8.8.4.4".to_string());
    let mut domain_name = use_signal(String::new);
    let mut lease_time = use_signal(|| "3600".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        if subnet().is_empty() || range_start().is_empty() || range_end().is_empty() {
            error.set(Some(
                "Subnet, range start, and range end are required".to_string(),
            ));
            return;
        }
        let lt: u32 = match lease_time().parse() {
            Ok(t) => t,
            Err(_) => {
                error.set(Some("Lease time must be a number (seconds)".to_string()));
                return;
            }
        };

        submitting.set(true);
        error.set(None);

        let dns: Vec<String> = dns_servers()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let pool = DhcpPool {
            id: 0,
            interface: interface(),
            enabled: true,
            subnet: subnet(),
            range_start: range_start(),
            range_end: range_end(),
            gateway: if gateway().is_empty() {
                None
            } else {
                Some(gateway())
            },
            dns_servers: dns,
            domain_name: if domain_name().is_empty() {
                None
            } else {
                Some(domain_name())
            },
            lease_time: lt,
        };

        spawn(async move {
            match api_client::post::<DhcpPool, DhcpPool>("/dhcp/pools", &pool).await {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Create DHCP Pool" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Interface" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "eth1", value: "{interface}",
                        oninput: move |e| interface.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Subnet (CIDR)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{subnet}",
                        oninput: move |e| subnet.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Range Start" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.100", value: "{range_start}",
                        oninput: move |e| range_start.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Range End" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.200", value: "{range_end}",
                        oninput: move |e| range_end.set(e.value()),
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
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "DNS Servers" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "8.8.8.8, 8.8.4.4", value: "{dns_servers}",
                        oninput: move |e| dns_servers.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Domain Name" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "local", value: "{domain_name}",
                        oninput: move |e| domain_name.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Lease Time (sec)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "number", placeholder: "3600", value: "{lease_time}",
                        oninput: move |e| lease_time.set(e.value()),
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Create Pool" }
            }
        }
    }
}

#[component]
fn DhcpReservationForm(on_saved: EventHandler<()>) -> Element {
    let mut mac = use_signal(String::new);
    let mut ip = use_signal(String::new);
    let mut hostname = use_signal(String::new);
    let mut pool_id = use_signal(|| "1".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        if mac().is_empty() || ip().is_empty() {
            error.set(Some("MAC address and IP address are required".to_string()));
            return;
        }
        let pid: u32 = pool_id().parse().unwrap_or(1);
        submitting.set(true);
        error.set(None);

        let reservation = DhcpReservation {
            id: 0,
            pool_id: pid,
            mac: mac(),
            ip: ip(),
            hostname: if hostname().is_empty() {
                None
            } else {
                Some(hostname())
            },
        };

        spawn(async move {
            match api_client::post::<DhcpReservation, DhcpReservation>(
                "/dhcp/reservations",
                &reservation,
            )
            .await
            {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Create Static Reservation" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "MAC Address" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "aa:bb:cc:dd:ee:ff", value: "{mac}",
                        oninput: move |e| mac.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "IP Address" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.50", value: "{ip}",
                        oninput: move |e| ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Hostname" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "my-server", value: "{hostname}",
                        oninput: move |e| hostname.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Pool ID" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "number", placeholder: "1", value: "{pool_id}",
                        oninput: move |e| pool_id.set(e.value()),
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Create Reservation" }
            }
        }
    }
}

#[component]
fn DhcpClientForm(on_saved: EventHandler<()>) -> Element {
    let mut interface = use_signal(|| "eth0".to_string());
    let mut hostname = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        if interface().is_empty() {
            error.set(Some("Interface is required".to_string()));
            return;
        }
        submitting.set(true);
        error.set(None);

        let config = DhcpClientConfig {
            id: 0,
            interface: interface(),
            enabled: true,
            hostname: if hostname().is_empty() {
                None
            } else {
                Some(hostname())
            },
        };

        spawn(async move {
            match api_client::post::<DhcpClientConfig, DhcpClientConfig>("/dhcp/clients", &config)
                .await
            {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Add WAN DHCP Client" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "WAN Interface" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "eth0", value: "{interface}",
                        oninput: move |e| interface.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Hostname (optional)" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "nylon-wall", value: "{hostname}",
                        oninput: move |e| hostname.set(e.value()),
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Add Client" }
            }
        }
    }
}

// === Helpers ===

fn format_lease_time(seconds: u32) -> String {
    if seconds >= 86400 {
        format!("{}d", seconds / 86400)
    } else if seconds >= 3600 {
        format!("{}h", seconds / 3600)
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}

fn format_timestamp(lease_end: i64) -> String {
    // Show remaining lease duration relative to lease_start
    // Since we don't have direct access to JS Date in pure Rust,
    // just display a human-readable absolute time
    let dt = chrono::DateTime::from_timestamp(lease_end, 0);
    match dt {
        Some(d) => d.format("%Y-%m-%d %H:%M").to_string(),
        None => "Unknown".to_string(),
    }
}
