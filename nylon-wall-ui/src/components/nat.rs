use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn Nat() -> Element {
    let mut entries = use_resource(|| async { api_client::get::<Vec<NatEntry>>("/nat").await });
    let mut show_form = use_signal(|| false);
    let mut show_wizard = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "flex items-center justify-between mb-6",
                div {
                    h2 { class: "text-xl font-semibold text-white", "NAT Configuration" }
                    p { class: "text-sm text-slate-400 mt-1", "Network address translation entries" }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20 hover:bg-violet-500/20 transition-colors",
                        onclick: move |_| {
                            show_wizard.set(!show_wizard());
                            if show_wizard() { show_form.set(false); }
                        },
                        if show_wizard() {
                            "Cancel"
                        } else {
                            Icon { width: 12, height: 12, icon: LdArrowRightLeft }
                            span { "Port Forward Wizard" }
                        }
                    }
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
                        onclick: move |_| {
                            show_form.set(!show_form());
                            if show_form() { show_wizard.set(false); }
                        },
                        if show_form() { "Cancel" } else { "+ New NAT Entry" }
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

            if show_wizard() {
                PortForwardWizard {
                    on_saved: move |_| {
                        show_wizard.set(false);
                        entries.restart();
                    }
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

            div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
                table { class: "w-full text-left",
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Type" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Source" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Destination" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Protocol" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Translate To" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Interface" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "Status" }
                            th { class: "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500", "" }
                        }
                    }
                    tbody {
                        match &*entries.read() {
                            Some(Ok(list)) if !list.is_empty() => rsx! {
                                for entry in list.iter() {
                                    tr { class: "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors",
                                        key: "{entry.id}",
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: match entry.nat_type {
                                                    NatType::SNAT => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20",
                                                    NatType::DNAT => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20",
                                                    NatType::Masquerade => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-cyan-500/10 text-cyan-400 border border-cyan-500/20",
                                                },
                                                match entry.nat_type {
                                                    NatType::SNAT => "SNAT",
                                                    NatType::DNAT => "DNAT",
                                                    NatType::Masquerade => "MASQ",
                                                }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono", {entry.src_network.clone().unwrap_or("*".to_string())} }
                                        td { class: "px-5 py-3 text-sm text-slate-400 font-mono", {entry.dst_network.clone().unwrap_or("*".to_string())} }
                                        td { class: "px-5 py-3 text-sm text-slate-400", {entry.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                        td { class: "px-5 py-3 text-sm text-slate-300 font-mono",
                                            {format_translate(entry)}
                                        }
                                        td { class: "px-5 py-3 text-sm text-slate-400", {entry.out_interface.clone().unwrap_or("\u{2014}".to_string())} }
                                        td { class: "px-5 py-3 text-sm",
                                            span {
                                                class: if entry.enabled {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                                                } else {
                                                    "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20"
                                                },
                                                if entry.enabled { "Enabled" } else { "Disabled" }
                                            }
                                        }
                                        td { class: "px-5 py-3 text-sm",
                                            {
                                                let id = entry.id;
                                                rsx! {
                                                    button {
                                                        class: "px-2 py-0.5 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::delete(&format!("/nat/{}", id)).await {
                                                                    Ok(_) => entries.restart(),
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
                                tr { td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "8", "No NAT entries configured" } }
                            },
                            Some(Err(e)) => rsx! {
                                tr { td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "8", "Failed to load NAT entries: {e}" } }
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

fn format_translate(entry: &NatEntry) -> String {
    let ip = entry.translate_ip.clone().unwrap_or("\u{2014}".to_string());
    match &entry.translate_port {
        Some(pr) if pr.start == pr.end => format!("{}:{}", ip, pr.start),
        Some(pr) => format!("{}:{}-{}", ip, pr.start, pr.end),
        None => ip,
    }
}

// === Port Forward Wizard ===

#[component]
fn PortForwardWizard(on_saved: EventHandler<()>) -> Element {
    let mut ext_port = use_signal(|| String::new());
    let mut int_ip = use_signal(|| String::new());
    let mut int_port = use_signal(|| String::new());
    let mut protocol = use_signal(|| "TCP".to_string());
    let mut in_interface = use_signal(|| "eth0".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        // Validate
        let ext: u16 = match ext_port().parse() {
            Ok(p) => p,
            Err(_) => {
                error.set(Some("External port must be a number (1-65535)".to_string()));
                return;
            }
        };
        if int_ip().is_empty() {
            error.set(Some("Internal IP address is required".to_string()));
            return;
        }
        let internal: u16 = if int_port().is_empty() {
            ext
        } else {
            match int_port().parse() {
                Ok(p) => p,
                Err(_) => {
                    error.set(Some("Internal port must be a number (1-65535)".to_string()));
                    return;
                }
            }
        };

        submitting.set(true);
        error.set(None);

        let entry = NatEntry {
            id: 0,
            nat_type: NatType::DNAT,
            enabled: true,
            src_network: None,
            dst_network: None,
            protocol: match protocol().as_str() {
                "UDP" => Some(Protocol::UDP),
                "Both" => None,
                _ => Some(Protocol::TCP),
            },
            dst_port: Some(PortRange::single(ext)),
            in_interface: Some(in_interface()),
            out_interface: None,
            translate_ip: Some(int_ip()),
            translate_port: Some(PortRange::single(internal)),
        };

        spawn(async move {
            match api_client::post::<NatEntry, NatEntry>("/nat", &entry).await {
                Ok(_) => {
                    // If "Both" protocol, create a second entry for UDP
                    if protocol() == "Both" {
                        let udp_entry = NatEntry {
                            id: 0,
                            nat_type: NatType::DNAT,
                            enabled: true,
                            src_network: None,
                            dst_network: None,
                            protocol: Some(Protocol::UDP),
                            dst_port: Some(PortRange::single(ext)),
                            in_interface: Some(in_interface()),
                            out_interface: None,
                            translate_ip: Some(int_ip()),
                            translate_port: Some(PortRange::single(internal)),
                        };
                        let _ = api_client::post::<NatEntry, NatEntry>("/nat", &udp_entry).await;
                    }
                    on_saved.call(());
                }
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-violet-500/20 bg-violet-500/5 p-6 mb-6",
            div { class: "flex items-center gap-2 mb-4",
                Icon { width: 18, height: 18, icon: LdArrowRightLeft, class: "text-violet-400" }
                h3 { class: "text-sm font-semibold text-white", "Port Forward Wizard" }
            }
            p { class: "text-xs text-slate-400 mb-4", "Forward an external port to an internal server. Creates a DNAT rule automatically." }

            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }

            // Visual diagram
            div { class: "flex items-center justify-center gap-3 mb-5 py-4 rounded-lg bg-slate-900/50 border border-slate-800/40",
                div { class: "text-center",
                    div { class: "text-[11px] text-slate-500 mb-1", "Internet" }
                    div { class: "px-3 py-2 rounded-lg bg-blue-500/10 border border-blue-500/20",
                        Icon { width: 20, height: 20, icon: LdGlobe, class: "text-blue-400 mx-auto" }
                    }
                }
                div { class: "flex items-center gap-1",
                    div { class: "text-xs text-slate-500 font-mono",
                        if !ext_port().is_empty() { ":{ext_port}" } else { ":?" }
                    }
                    Icon { width: 16, height: 16, icon: LdArrowRight, class: "text-violet-400" }
                }
                div { class: "text-center",
                    div { class: "text-[11px] text-slate-500 mb-1", "Firewall" }
                    div { class: "px-3 py-2 rounded-lg bg-violet-500/10 border border-violet-500/20",
                        Icon { width: 20, height: 20, icon: LdShield, class: "text-violet-400 mx-auto" }
                    }
                }
                div { class: "flex items-center gap-1",
                    Icon { width: 16, height: 16, icon: LdArrowRight, class: "text-violet-400" }
                    div { class: "text-xs text-slate-500 font-mono",
                        if !int_ip().is_empty() {
                            if !int_port().is_empty() { "{int_ip}:{int_port}" } else if !ext_port().is_empty() { "{int_ip}:{ext_port}" } else { "{int_ip}:?" }
                        } else {
                            "?:?"
                        }
                    }
                }
                div { class: "text-center",
                    div { class: "text-[11px] text-slate-500 mb-1", "Server" }
                    div { class: "px-3 py-2 rounded-lg bg-emerald-500/10 border border-emerald-500/20",
                        Icon { width: 20, height: 20, icon: LdServer, class: "text-emerald-400 mx-auto" }
                    }
                }
            }

            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "External Port" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-violet-500/60 transition-colors",
                        r#type: "number", placeholder: "e.g. 8080",
                        value: "{ext_port}",
                        oninput: move |e| ext_port.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Internal IP" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-violet-500/60 transition-colors",
                        r#type: "text", placeholder: "e.g. 192.168.1.50",
                        value: "{int_ip}",
                        oninput: move |e| int_ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Internal Port" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-violet-500/60 transition-colors",
                        r#type: "number", placeholder: "Same as external",
                        value: "{int_port}",
                        oninput: move |e| int_port.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Protocol" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-violet-500/60 transition-colors",
                        value: "{protocol}", onchange: move |e| protocol.set(e.value()),
                        option { value: "TCP", "TCP" }
                        option { value: "UDP", "UDP" }
                        option { value: "Both", "TCP + UDP" }
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "WAN Interface" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-violet-500/60 transition-colors",
                        r#type: "text", placeholder: "eth0",
                        value: "{in_interface}",
                        oninput: move |e| in_interface.set(e.value()),
                    }
                }
            }

            button {
                class: "flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20 hover:bg-violet-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() {
                    "Creating..."
                } else {
                    Icon { width: 14, height: 14, icon: LdArrowRightLeft }
                    span { "Create Port Forward" }
                }
            }
        }
    }
}

// === NAT Form (advanced) ===

#[component]
fn NatForm(on_saved: EventHandler<()>) -> Element {
    let mut nat_type = use_signal(|| "SNAT".to_string());
    let mut src_network = use_signal(|| String::new());
    let mut dst_network = use_signal(|| String::new());
    let mut translate_ip = use_signal(|| String::new());
    let mut out_interface = use_signal(|| String::new());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
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
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6",
            h3 { class: "text-sm font-semibold text-white mb-4", "Create NAT Entry" }
            if let Some(err) = error() {
                div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400", "{err}" }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "NAT Type" }
                    select {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        value: "{nat_type}", onchange: move |e| nat_type.set(e.value()),
                        option { value: "SNAT", "SNAT" }
                        option { value: "DNAT", "DNAT" }
                        option { value: "Masquerade", "Masquerade" }
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Source Network" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{src_network}",
                        oninput: move |e| src_network.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Destination Network" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "0.0.0.0/0", value: "{dst_network}",
                        oninput: move |e| dst_network.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Translate IP" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "203.0.113.1", value: "{translate_ip}",
                        oninput: move |e| translate_ip.set(e.value()),
                    }
                }
                div {
                    label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "Out Interface" }
                    input {
                        class: "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors",
                        r#type: "text", placeholder: "eth0", value: "{out_interface}",
                        oninput: move |e| out_interface.set(e.value()),
                    }
                }
            }
            button {
                class: "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
                disabled: submitting(),
                onclick: on_submit,
                if submitting() { "Creating..." } else { "Create Entry" }
            }
        }
    }
}
