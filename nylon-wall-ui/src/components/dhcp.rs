use super::ConfirmModal;
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn Dhcp() -> Element {
    let mut active_tab = use_signal(|| 0u8); // 0=Pools, 1=Leases, 2=Client

    let pools = use_resource(|| async { api_client::get::<Vec<DhcpPool>>("/dhcp/pools").await });
    let leases = use_resource(|| async { api_client::get::<Vec<DhcpLease>>("/dhcp/leases").await });
    let reservations = use_resource(|| async {
        api_client::get::<Vec<DhcpReservation>>("/dhcp/reservations").await
    });
    let clients =
        use_resource(|| async { api_client::get::<Vec<DhcpClientConfig>>("/dhcp/clients").await });

    // Summary counts
    let pool_count = match &*pools.read() {
        Some(Ok(p)) => p.len(),
        _ => 0,
    };
    let pool_active = match &*pools.read() {
        Some(Ok(p)) => p.iter().filter(|p| p.enabled).count(),
        _ => 0,
    };
    let lease_count = match &*leases.read() {
        Some(Ok(l)) => l
            .iter()
            .filter(|l| l.state == DhcpLeaseState::Active)
            .count(),
        _ => 0,
    };
    let reservation_count = match &*reservations.read() {
        Some(Ok(r)) => r.len(),
        _ => 0,
    };
    let client_count = match &*clients.read() {
        Some(Ok(c)) => c.len(),
        _ => 0,
    };
    let client_active = match &*clients.read() {
        Some(Ok(c)) => c.iter().filter(|c| c.enabled).count(),
        _ => 0,
    };

    rsx! {
        div {
            // Page header
            PageHeader {
                title: "DHCP",
                subtitle: "DHCP server pools, leases, and WAN client configuration",
            }

            // Summary stat cards
            div { class: "grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8",
                StatCard {
                    color: Color::Teal,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdServer, class: "text-teal-400" } },
                    label: "Pools",
                    value: "{pool_count}",
                    subtitle: "{pool_active} active",
                }
                StatCard {
                    color: Color::Emerald,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdCable, class: "text-emerald-400" } },
                    label: "Leases",
                    value: "{lease_count}",
                    subtitle: "active",
                }
                StatCard {
                    color: Color::Blue,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdBookmark, class: "text-blue-400" } },
                    label: "Reservations",
                    value: "{reservation_count}",
                    subtitle: "static",
                }
                StatCard {
                    color: Color::Violet,
                    icon: rsx! { Icon { width: 16, height: 16, icon: LdWifi, class: "text-violet-400" } },
                    label: "WAN Clients",
                    value: "{client_count}",
                    subtitle: "{client_active} active",
                }
            }

            // Tab navigation
            div { class: "flex gap-1 mb-6 bg-slate-900/50 rounded-xl p-1 border border-slate-800/40 w-fit",
                button {
                    class: if active_tab() == 0 {
                        "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 ring-1 ring-blue-500/20"
                    } else {
                        "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
                    },
                    onclick: move |_| active_tab.set(0),
                    Icon { width: 13, height: 13, icon: LdServer }
                    "Server Pools"
                }
                button {
                    class: if active_tab() == 1 {
                        "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 ring-1 ring-blue-500/20"
                    } else {
                        "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
                    },
                    onclick: move |_| active_tab.set(1),
                    Icon { width: 13, height: 13, icon: LdList }
                    "Leases"
                }
                button {
                    class: if active_tab() == 2 {
                        "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 ring-1 ring-blue-500/20"
                    } else {
                        "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
                    },
                    onclick: move |_| active_tab.set(2),
                    Icon { width: 13, height: 13, icon: LdWifi }
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

// === Tab 1: Server Pools (Card-based) ===

#[component]
fn DhcpPoolsTab() -> Element {
    let mut pools =
        use_resource(|| async { api_client::get::<Vec<DhcpPool>>("/dhcp/pools").await });
    let mut editing = use_signal(|| None::<(bool, DhcpPool)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<u32>);

    rsx! {
        div {
            SectionHeader {
                icon: rsx! { Icon { width: 13, height: 13, icon: LdServer, class: "text-teal-400" } },
                title: "DHCP Server Pools",
                div { class: "flex items-center justify-between gap-2 ",
                    RefreshBtn { onclick: move |_| pools.restart() }
                    Btn {
                        color: Color::Blue,
                        label: if editing().is_some() { "Cancel".to_string() } else { "+ New Pool".to_string() },
                        onclick: move |_| {
                            if editing().is_some() {
                                editing.set(None);
                            } else {
                                editing.set(Some((false, DhcpPool {
                                    id: 0, interface: "eth1".to_string(), enabled: true,
                                    subnet: String::new(), range_start: String::new(),
                                    range_end: String::new(), gateway: None,
                                    dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
                                    domain_name: None, lease_time: 3600,
                                })));
                            }
                        },
                    }
                }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            if let Some((is_edit, pool)) = editing() {
                DhcpPoolForm {
                    key: "{pool.id}",
                    is_edit: is_edit,
                    editing: pool,
                    on_saved: move |_| {
                        editing.set(None);
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

            match &*pools.read() {
                Some(Ok(list)) if !list.is_empty() => rsx! {
                    div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
                        for pool in list.iter() {
                            {
                                let pool_id = pool.id;
                                let is_enabled = pool.enabled;
                                let pool_clone = pool.clone();
                                rsx! {
                                    div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-slate-700/60 transition-colors",
                                        key: "{pool.id}",
                                        // Card header
                                        div { class: "flex items-center justify-between mb-4",
                                            div { class: "flex items-center gap-2.5",
                                                div { class: "w-8 h-8 rounded-lg bg-teal-500/10 flex items-center justify-center",
                                                    Icon { width: 14, height: 14, icon: LdServer, class: "text-teal-400" }
                                                }
                                                div {
                                                    span { class: "text-sm font-semibold text-white", "{pool.interface}" }
                                                    p { class: "text-xs text-slate-500 font-mono", "{pool.subnet}" }
                                                }
                                            }
                                            div { class: "flex items-center gap-2",
                                                button {
                                                    class: if is_enabled {
                                                        "px-2.5 py-1 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 cursor-pointer hover:bg-emerald-500/20 transition-colors"
                                                    } else {
                                                        "px-2.5 py-1 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20 cursor-pointer hover:bg-slate-500/20 transition-colors"
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

                                        // Card body - key/value grid
                                        div { class: "grid grid-cols-2 gap-x-6 gap-y-2 text-xs mb-4",
                                            div { class: "text-slate-500", "IP Range" }
                                            div { class: "text-slate-300 font-mono",
                                                "{pool.range_start} \u{2013} {pool.range_end}"
                                            }
                                            div { class: "text-slate-500", "Gateway" }
                                            div { class: "text-slate-300 font-mono",
                                                {pool.gateway.clone().unwrap_or("\u{2014}".to_string())}
                                            }
                                            div { class: "text-slate-500", "DNS Servers" }
                                            div { class: "text-slate-300 font-mono",
                                                {
                                                    if pool.dns_servers.is_empty() {
                                                        "\u{2014}".to_string()
                                                    } else {
                                                        pool.dns_servers.join(", ")
                                                    }
                                                }
                                            }
                                            if let Some(ref domain) = pool.domain_name {
                                                div { class: "text-slate-500", "Domain" }
                                                div { class: "text-slate-300 font-mono", "{domain}" }
                                            }
                                            div { class: "text-slate-500", "Lease Time" }
                                            div { class: "text-slate-300", {format_lease_time(pool.lease_time)} }
                                        }

                                        // Card footer
                                        div { class: "flex items-center justify-end gap-1 pt-3 border-t border-slate-800/40",
                                            EditBtn {
                                                onclick: move |_| editing.set(Some((true, pool_clone.clone()))),
                                            }
                                            {
                                                let id = pool.id;
                                                rsx! {
                                                    DeleteBtn {
                                                        onclick: move |_| confirm_delete.set(Some(id)),
                                                    }
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
                    EmptyState {
                        icon: rsx! { Icon { width: 24, height: 24, icon: LdServer, class: "text-slate-600" } },
                        title: "No DHCP pools configured",
                        subtitle: "Create a pool to start serving DHCP addresses on your network interfaces",
                        div {
                            Btn {
                                color: Color::Blue,
                                label: "+ Create First Pool",
                                onclick: move |_| {
                                    editing.set(Some((false, DhcpPool {
                                        id: 0, interface: "eth1".to_string(), enabled: true,
                                        subnet: String::new(), range_start: String::new(),
                                        range_end: String::new(), gateway: None,
                                        dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
                                        domain_name: None, lease_time: 3600,
                                    })));
                                },
                            }
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "rounded-xl border border-red-500/20 bg-red-500/5 p-8 text-center",
                        div { class: "flex justify-center mb-3",
                            div { class: "w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center",
                                Icon { width: 20, height: 20, icon: LdTriangleAlert, class: "text-red-400" }
                            }
                        }
                        p { class: "text-sm text-red-400", "Failed to load pools: {e}" }
                    }
                },
                None => rsx! {
                    div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
                        for _ in 0..2 {
                            div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 animate-pulse",
                                div { class: "flex items-center gap-2.5 mb-4",
                                    div { class: "w-8 h-8 rounded-lg bg-slate-800/80" }
                                    div {
                                        div { class: "w-16 h-3.5 rounded bg-slate-800/80 mb-1.5" }
                                        div { class: "w-24 h-3 rounded bg-slate-800/60" }
                                    }
                                }
                                div { class: "space-y-2",
                                    div { class: "w-full h-3 rounded bg-slate-800/60" }
                                    div { class: "w-3/4 h-3 rounded bg-slate-800/60" }
                                    div { class: "w-1/2 h-3 rounded bg-slate-800/60" }
                                }
                            }
                        }
                    }
                },
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
    let mut editing_reservation = use_signal(|| None::<(bool, DhcpReservation)>);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_release = use_signal(|| None::<String>); // MAC to release

    rsx! {
        div {
            // Active Leases section
            SectionHeader {
                icon: rsx! { Icon { width: 13, height: 13, icon: LdCable, class: "text-emerald-400" } },
                title: "Active Leases",
                RefreshBtn { onclick: move |_| { leases.restart(); reservations.restart(); } }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
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

            match &*leases.read() {
                Some(Ok(list)) if !list.is_empty() => rsx! {
                    div { class: "mb-8",
                        DataTable {
                            thead { class: "bg-slate-900/80",
                                tr {
                                    th { class: TH_CLASS, "IP Address" }
                                    th { class: TH_CLASS, "MAC Address" }
                                    th { class: TH_CLASS, "Hostname" }
                                    th { class: TH_CLASS, "Expires" }
                                    th { class: TH_CLASS, "State" }
                                    th { class: TH_CLASS, "" }
                                }
                            }
                            tbody {
                                for lease in list.iter() {
                                    tr { class: TR_CLASS,
                                        key: "{lease.mac}",
                                        td { class: "{TD_CLASS} text-slate-300 font-mono", "{lease.ip}" }
                                        td { class: "{TD_CLASS} text-slate-400 font-mono text-xs", "{lease.mac}" }
                                        td { class: "{TD_CLASS} text-slate-400",
                                            {lease.hostname.clone().unwrap_or("\u{2014}".to_string())}
                                        }
                                        td { class: "{TD_CLASS} text-slate-400 text-xs",
                                            {format_timestamp(lease.lease_end)}
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: match lease.state {
                                                    DhcpLeaseState::Active => Color::Emerald,
                                                    DhcpLeaseState::Reserved => Color::Blue,
                                                    DhcpLeaseState::Expired => Color::Slate,
                                                },
                                                label: match lease.state {
                                                    DhcpLeaseState::Active => "Active".to_string(),
                                                    DhcpLeaseState::Reserved => "Reserved".to_string(),
                                                    DhcpLeaseState::Expired => "Expired".to_string(),
                                                },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            {
                                                let mac = lease.mac.clone();
                                                let mac2 = lease.mac.clone();
                                                rsx! {
                                                    div { class: "flex items-center gap-1",
                                                        button {
                                                            class: "p-1.5 rounded-lg text-blue-400 hover:bg-blue-500/10 transition-colors",
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
                                                            class: "p-1.5 rounded-lg text-red-400 hover:bg-red-500/10 transition-colors",
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
                            }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    div { class: "mb-8",
                        EmptyState {
                            icon: rsx! { Icon { width: 20, height: 20, icon: LdCable, class: "text-slate-600" } },
                            title: "No active DHCP leases",
                            subtitle: "Leases will appear here when clients receive addresses from your DHCP pools",
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "rounded-xl border border-red-500/20 bg-red-500/5 p-8 text-center mb-8",
                        div { class: "flex justify-center mb-3",
                            div { class: "w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center",
                                Icon { width: 20, height: 20, icon: LdTriangleAlert, class: "text-red-400" }
                            }
                        }
                        p { class: "text-sm text-red-400", "Failed to load leases: {e}" }
                    }
                },
                None => rsx! {
                    div { class: "rounded-xl border border-slate-800/60 overflow-hidden mb-8 animate-pulse",
                        div { class: "bg-slate-900/80 px-5 py-3",
                            div { class: "flex gap-12",
                                div { class: "w-20 h-3 rounded bg-slate-800/80" }
                                div { class: "w-24 h-3 rounded bg-slate-800/80" }
                                div { class: "w-16 h-3 rounded bg-slate-800/80" }
                            }
                        }
                        div { class: "px-5 py-8 text-center",
                            div { class: "w-32 h-3 rounded bg-slate-800/60 mx-auto" }
                        }
                    }
                },
            }

            // Reservations section
            SectionHeader {
                icon: rsx! { Icon { width: 13, height: 13, icon: LdBookmark, class: "text-blue-400" } },
                title: "Static Reservations",
                Btn {
                    color: Color::Blue,
                    label: if editing_reservation().is_some() { "Cancel".to_string() } else { "+ New Reservation".to_string() },
                    onclick: move |_| {
                        if editing_reservation().is_some() {
                            editing_reservation.set(None);
                        } else {
                            editing_reservation.set(Some((false, DhcpReservation {
                                id: 0, pool_id: 1, mac: String::new(),
                                ip: String::new(), hostname: None,
                            })));
                        }
                    },
                }
            }

            if let Some((is_edit, res)) = editing_reservation() {
                DhcpReservationForm {
                    key: "{res.id}",
                    is_edit: is_edit,
                    editing: res,
                    on_saved: move |_| {
                        editing_reservation.set(None);
                        reservations.restart();
                    }
                }
            }

            match &*reservations.read() {
                Some(Ok(list)) if !list.is_empty() => rsx! {
                    DataTable {
                        thead { class: "bg-slate-900/80",
                            tr {
                                th { class: TH_CLASS, "MAC Address" }
                                th { class: TH_CLASS, "IP Address" }
                                th { class: TH_CLASS, "Hostname" }
                                th { class: TH_CLASS, "Pool" }
                                th { class: TH_CLASS, "" }
                            }
                        }
                        tbody {
                            for res in list.iter() {
                                tr { class: TR_CLASS,
                                    key: "{res.id}",
                                    td { class: "{TD_CLASS} text-slate-400 font-mono text-xs", "{res.mac}" }
                                    td { class: "{TD_CLASS} text-slate-300 font-mono", "{res.ip}" }
                                    td { class: "{TD_CLASS} text-slate-400",
                                        {res.hostname.clone().unwrap_or("\u{2014}".to_string())}
                                    }
                                    td { class: "{TD_CLASS} text-slate-400", "#{res.pool_id}" }
                                    td { class: TD_CLASS,
                                        {
                                            let res_clone = res.clone();
                                            let id = res.id;
                                            rsx! {
                                                div { class: "flex items-center gap-1",
                                                    EditBtn {
                                                        onclick: move |_| {
                                                            editing_reservation.set(Some((true, res_clone.clone())));
                                                        },
                                                    }
                                                    button {
                                                        class: "p-1.5 rounded-lg text-red-400 hover:bg-red-500/10 transition-colors",
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
                            }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    EmptyState {
                        icon: rsx! { Icon { width: 20, height: 20, icon: LdBookmark, class: "text-slate-600" } },
                        title: "No static reservations",
                        subtitle: "Reserve fixed IP addresses for specific devices by MAC address",
                        Btn {
                            color: Color::Blue,
                            label: "+ Create Reservation",
                            onclick: move |_| {
                                editing_reservation.set(Some((false, DhcpReservation {
                                    id: 0, pool_id: 1, mac: String::new(),
                                    ip: String::new(), hostname: None,
                                })));
                            },
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "rounded-xl border border-red-500/20 bg-red-500/5 p-8 text-center",
                        div { class: "flex justify-center mb-3",
                            div { class: "w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center",
                                Icon { width: 20, height: 20, icon: LdTriangleAlert, class: "text-red-400" }
                            }
                        }
                        p { class: "text-sm text-red-400", "Failed to load reservations: {e}" }
                    }
                },
                None => rsx! {
                    div { class: "rounded-xl border border-slate-800/60 overflow-hidden animate-pulse",
                        div { class: "bg-slate-900/80 px-5 py-3",
                            div { class: "flex gap-12",
                                div { class: "w-24 h-3 rounded bg-slate-800/80" }
                                div { class: "w-20 h-3 rounded bg-slate-800/80" }
                                div { class: "w-16 h-3 rounded bg-slate-800/80" }
                            }
                        }
                        div { class: "px-5 py-8 text-center",
                            div { class: "w-32 h-3 rounded bg-slate-800/60 mx-auto" }
                        }
                    }
                },
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
    let mut editing = use_signal(|| None::<(bool, DhcpClientConfig)>);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div {
            SectionHeader {
                icon: rsx! { Icon { width: 13, height: 13, icon: LdWifi, class: "text-violet-400" } },
                title: "WAN DHCP Clients",
                div { class: "flex items-center gap-2",
                    RefreshBtn { onclick: move |_| { clients.restart(); statuses.restart(); } }
                    Btn {
                        color: Color::Blue,
                        label: if editing().is_some() { "Cancel".to_string() } else { "+ Add WAN Client".to_string() },
                        onclick: move |_| {
                            if editing().is_some() {
                                editing.set(None);
                            } else {
                                editing.set(Some((false, DhcpClientConfig {
                                    id: 0, interface: "eth0".to_string(),
                                    enabled: true, hostname: None,
                                })));
                            }
                        },
                    }
                }
            }

            if let Some(err) = error_msg() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error_msg.set(None),
                }
            }

            if let Some((is_edit, config)) = editing() {
                DhcpClientForm {
                    key: "{config.id}",
                    is_edit: is_edit,
                    editing: config,
                    on_saved: move |_| {
                        editing.set(None);
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
                                    let config_clone = config.clone();
                                    rsx! {
                                        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-slate-700/60 transition-colors",
                                            key: "{config.id}",
                                            // Card header
                                            div { class: "flex items-center justify-between mb-4",
                                                div { class: "flex items-center gap-2.5",
                                                    div { class: "w-8 h-8 rounded-lg bg-violet-500/10 flex items-center justify-center",
                                                        Icon { width: 14, height: 14, icon: LdWifi, class: "text-violet-400" }
                                                    }
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
                                                                class: format!("px-2.5 py-1 rounded-full text-[11px] font-medium border {}", state_badge.0),
                                                                "{state_badge.1}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // Status details
                                            if let Some(ref s) = status {
                                                div { class: "grid grid-cols-2 gap-x-6 gap-y-2 mb-4 text-xs",
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
                                                    div { class: "text-slate-500", "DNS Servers" }
                                                    div { class: "text-slate-300 font-mono",
                                                        {
                                                            if s.dns_servers.is_empty() {
                                                                "\u{2014}".to_string()
                                                            } else {
                                                                s.dns_servers.join(", ")
                                                            }
                                                        }
                                                    }
                                                    div { class: "text-slate-500", "DHCP Server" }
                                                    div { class: "text-slate-300 font-mono",
                                                        {s.dhcp_server.clone().unwrap_or("\u{2014}".to_string())}
                                                    }
                                                }
                                            }

                                            // Actions
                                            div { class: "flex items-center gap-2 pt-3 border-t border-slate-800/40",
                                                Btn {
                                                    color: if is_enabled { Color::Emerald } else { Color::Slate },
                                                    label: if is_enabled { "Disable".to_string() } else { "Enable".to_string() },
                                                    onclick: move |_| {
                                                        spawn(async move {
                                                            match api_client::post::<(), serde_json::Value>(&format!("/dhcp/clients/{}/toggle", config_id), &()).await {
                                                                Ok(_) => { clients.restart(); statuses.restart(); },
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                }
                                                Btn {
                                                    color: Color::Blue,
                                                    label: "Renew",
                                                    onclick: move |_| {
                                                        let iface_val = iface.clone();
                                                        spawn(async move {
                                                            match api_client::post::<(), serde_json::Value>(&format!("/dhcp/clients/{}/renew", iface_val), &()).await {
                                                                Ok(_) => statuses.restart(),
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                }
                                                Btn {
                                                    color: Color::Amber,
                                                    label: "Release",
                                                    onclick: move |_| {
                                                        let iface_val = iface2.clone();
                                                        spawn(async move {
                                                            match api_client::post::<(), serde_json::Value>(&format!("/dhcp/clients/{}/release", iface_val), &()).await {
                                                                Ok(_) => statuses.restart(),
                                                                Err(e) => error_msg.set(Some(e)),
                                                            }
                                                        });
                                                    },
                                                }
                                                div { class: "ml-auto flex items-center gap-1",
                                                    EditBtn {
                                                        onclick: move |_| {
                                                            editing.set(Some((true, config_clone.clone())));
                                                        },
                                                    }
                                                    DeleteBtn {
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/dhcp/clients/{}", config_id)).await {
                                                                    Ok(_) => { clients.restart(); statuses.restart(); },
                                                                    Err(e) => error_msg.set(Some(e)),
                                                                }
                                                            });
                                                        },
                                                    }
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
                    EmptyState {
                        icon: rsx! { Icon { width: 24, height: 24, icon: LdWifi, class: "text-slate-600" } },
                        title: "No WAN DHCP clients configured",
                        subtitle: "Add a WAN client to obtain IP configuration from an upstream DHCP server",
                        Btn {
                            color: Color::Blue,
                            label: "+ Add WAN Client",
                            onclick: move |_| {
                                editing.set(Some((false, DhcpClientConfig {
                                    id: 0, interface: "eth0".to_string(),
                                    enabled: true, hostname: None,
                                })));
                            },
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "rounded-xl border border-red-500/20 bg-red-500/5 p-8 text-center",
                        div { class: "flex justify-center mb-3",
                            div { class: "w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center",
                                Icon { width: 20, height: 20, icon: LdTriangleAlert, class: "text-red-400" }
                            }
                        }
                        p { class: "text-sm text-red-400", "Failed to load DHCP clients: {e}" }
                    }
                },
                None => rsx! {
                    div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
                        for _ in 0..2 {
                            div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 animate-pulse",
                                div { class: "flex items-center gap-2.5 mb-4",
                                    div { class: "w-8 h-8 rounded-lg bg-slate-800/80" }
                                    div { class: "w-16 h-3.5 rounded bg-slate-800/80" }
                                }
                                div { class: "space-y-2",
                                    div { class: "w-full h-3 rounded bg-slate-800/60" }
                                    div { class: "w-3/4 h-3 rounded bg-slate-800/60" }
                                    div { class: "w-1/2 h-3 rounded bg-slate-800/60" }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

// === Forms ===

#[component]
fn DhcpPoolForm(is_edit: bool, editing: DhcpPool, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut interface = use_signal(|| editing.interface.clone());
    let mut subnet = use_signal(|| editing.subnet.clone());
    let mut range_start = use_signal(|| editing.range_start.clone());
    let mut range_end = use_signal(|| editing.range_end.clone());
    let mut gateway = use_signal(|| editing.gateway.clone().unwrap_or_default());
    let mut dns_servers = use_signal(|| editing.dns_servers.join(", "));
    let mut domain_name = use_signal(|| editing.domain_name.clone().unwrap_or_default());
    let mut lease_time = use_signal(|| editing.lease_time.to_string());
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
            id: edit_id,
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
            let result = if is_edit {
                api_client::put::<DhcpPool, DhcpPool>(&format!("/dhcp/pools/{}", edit_id), &pool).await
            } else {
                api_client::post::<DhcpPool, DhcpPool>("/dhcp/pools", &pool).await
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        FormCard { class: "rounded-xl border border-blue-500/20 bg-slate-900/80 p-6 mb-6",
            div { class: "flex items-center gap-2 mb-4",
                div { class: "w-7 h-7 rounded-lg bg-blue-500/10 flex items-center justify-center",
                    if is_edit {
                        Icon { width: 13, height: 13, icon: LdPencil, class: "text-blue-400" }
                    } else {
                        Icon { width: 13, height: 13, icon: LdPlus, class: "text-blue-400" }
                    }
                }
                h3 { class: "text-sm font-semibold text-white",
                    if is_edit { "Edit DHCP Pool" } else { "Create DHCP Pool" }
                }
            }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                InterfaceSelect {
                    value: interface(),
                    onchange: move |v| interface.set(v),
                }
                FormField { label: "Subnet (CIDR)",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{subnet}",
                        oninput: move |e| subnet.set(e.value()),
                    }
                }
                FormField { label: "Range Start",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.100", value: "{range_start}",
                        oninput: move |e| range_start.set(e.value()),
                    }
                }
                FormField { label: "Range End",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.200", value: "{range_end}",
                        oninput: move |e| range_end.set(e.value()),
                    }
                }
                FormField { label: "Gateway",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.1", value: "{gateway}",
                        oninput: move |e| gateway.set(e.value()),
                    }
                }
                FormField { label: "DNS Servers",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "8.8.8.8, 8.8.4.4", value: "{dns_servers}",
                        oninput: move |e| dns_servers.set(e.value()),
                    }
                }
                FormField { label: "Domain Name",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "local", value: "{domain_name}",
                        oninput: move |e| domain_name.set(e.value()),
                    }
                }
                FormField { label: "Lease Time (sec)",
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", placeholder: "3600", value: "{lease_time}",
                        oninput: move |e| lease_time.set(e.value()),
                    }
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Creating...".to_string() }
                } else {
                    if is_edit { "Save Pool".to_string() } else { "Create Pool".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}

#[component]
fn DhcpReservationForm(is_edit: bool, editing: DhcpReservation, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut mac = use_signal(|| editing.mac.clone());
    let mut ip = use_signal(|| editing.ip.clone());
    let mut hostname = use_signal(|| editing.hostname.clone().unwrap_or_default());
    let mut pool_id = use_signal(|| editing.pool_id.to_string());
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
            id: edit_id,
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
            let result = if is_edit {
                api_client::put::<DhcpReservation, DhcpReservation>(
                    &format!("/dhcp/reservations/{}", edit_id),
                    &reservation,
                ).await
            } else {
                api_client::post::<DhcpReservation, DhcpReservation>(
                    "/dhcp/reservations",
                    &reservation,
                ).await
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        FormCard { class: "rounded-xl border border-blue-500/20 bg-slate-900/80 p-6 mb-6",
            div { class: "flex items-center gap-2 mb-4",
                div { class: "w-7 h-7 rounded-lg bg-blue-500/10 flex items-center justify-center",
                    if is_edit {
                        Icon { width: 13, height: 13, icon: LdPencil, class: "text-blue-400" }
                    } else {
                        Icon { width: 13, height: 13, icon: LdPlus, class: "text-blue-400" }
                    }
                }
                h3 { class: "text-sm font-semibold text-white",
                    if is_edit { "Edit Reservation" } else { "Create Static Reservation" }
                }
            }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                FormField { label: "MAC Address",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "aa:bb:cc:dd:ee:ff", value: "{mac}",
                        oninput: move |e| mac.set(e.value()),
                    }
                }
                FormField { label: "IP Address",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.50", value: "{ip}",
                        oninput: move |e| ip.set(e.value()),
                    }
                }
                FormField { label: "Hostname",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "my-server", value: "{hostname}",
                        oninput: move |e| hostname.set(e.value()),
                    }
                }
                FormField { label: "Pool ID",
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", placeholder: "1", value: "{pool_id}",
                        oninput: move |e| pool_id.set(e.value()),
                    }
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Creating...".to_string() }
                } else {
                    if is_edit { "Save Reservation".to_string() } else { "Create Reservation".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}

#[component]
fn DhcpClientForm(is_edit: bool, editing: DhcpClientConfig, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut interface = use_signal(|| editing.interface.clone());
    let mut hostname = use_signal(|| editing.hostname.clone().unwrap_or_default());
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
            id: edit_id,
            interface: interface(),
            enabled: true,
            hostname: if hostname().is_empty() {
                None
            } else {
                Some(hostname())
            },
        };

        spawn(async move {
            let result = if is_edit {
                api_client::put::<DhcpClientConfig, DhcpClientConfig>(
                    &format!("/dhcp/clients/{}", edit_id),
                    &config,
                ).await
            } else {
                api_client::post::<DhcpClientConfig, DhcpClientConfig>("/dhcp/clients", &config).await
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        FormCard { class: "rounded-xl border border-blue-500/20 bg-slate-900/80 p-6 mb-6",
            div { class: "flex items-center gap-2 mb-4",
                div { class: "w-7 h-7 rounded-lg bg-blue-500/10 flex items-center justify-center",
                    if is_edit {
                        Icon { width: 13, height: 13, icon: LdPencil, class: "text-blue-400" }
                    } else {
                        Icon { width: 13, height: 13, icon: LdPlus, class: "text-blue-400" }
                    }
                }
                h3 { class: "text-sm font-semibold text-white",
                    if is_edit { "Edit WAN DHCP Client" } else { "Add WAN DHCP Client" }
                }
            }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4",
                InterfaceSelect {
                    value: interface(),
                    onchange: move |v| interface.set(v),
                    label: "WAN Interface",
                }
                FormField { label: "Hostname (optional)",
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "nylon-wall", value: "{hostname}",
                        oninput: move |e| hostname.set(e.value()),
                    }
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Creating...".to_string() }
                } else {
                    if is_edit { "Save Client".to_string() } else { "Add Client".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
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
    let dt = chrono::DateTime::from_timestamp(lease_end, 0);
    match dt {
        Some(d) => d.format("%Y-%m-%d %H:%M").to_string(),
        None => "Unknown".to_string(),
    }
}
