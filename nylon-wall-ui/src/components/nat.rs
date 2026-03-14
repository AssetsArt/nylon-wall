use super::{ConfirmModal, use_change_guard, notify_change};
use super::ui::*;
use crate::api_client;
use crate::models::*;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn Nat() -> Element {
    let mut entries = use_resource(|| async { api_client::get::<Vec<NatEntry>>("/nat").await });
    let mut editing = use_signal(|| None::<(bool, NatEntry)>);
    let mut show_wizard = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut guard = use_change_guard();

    rsx! {
        div {
            PageHeader {
                title: "NAT Configuration".to_string(),
                subtitle: "Network address translation entries".to_string(),
                div { class: "flex items-center gap-2",
                    if show_wizard() {
                        Btn {
                            color: Color::Violet,
                            label: "Cancel".to_string(),
                            onclick: move |_| {
                                show_wizard.set(false);
                            },
                        }
                    } else {
                        Btn {
                            color: Color::Violet,
                            label: "Port Forward Wizard".to_string(),
                            onclick: move |_| {
                                show_wizard.set(true);
                                editing.set(None);
                            },
                            icon: rsx! { Icon { width: 12, height: 12, icon: LdArrowRightLeft } },
                        }
                    }
                    Btn {
                        color: Color::Blue,
                        label: if editing().is_some() { "Cancel".to_string() } else { "+ New NAT Entry".to_string() },
                        onclick: move |_| {
                            if editing().is_some() {
                                editing.set(None);
                            } else {
                                editing.set(Some((false, NatEntry {
                                    id: 0, nat_type: NatType::SNAT, enabled: true,
                                    src_network: None, dst_network: None, protocol: None,
                                    dst_port: None, in_interface: None, out_interface: None,
                                    translate_ip: None, translate_port: None,
                                })));
                                show_wizard.set(false);
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

            if show_wizard() {
                PortForwardWizard {
                    on_saved: move |_| {
                        show_wizard.set(false);
                        entries.restart();
                        notify_change(&mut guard);
                    }
                }
            }

            if let Some((is_edit, entry)) = editing() {
                NatForm {
                    key: "{entry.id}",
                    is_edit: is_edit,
                    editing: entry,
                    on_saved: move |_| {
                        editing.set(None);
                        entries.restart();
                        notify_change(&mut guard);
                    }
                }
            }

            if let Some(del_id) = confirm_delete() {
                ConfirmModal {
                    title: "Delete NAT Entry".to_string(),
                    message: format!("Are you sure you want to delete NAT entry #{}? This action cannot be undone.", del_id),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: move |_| {
                        confirm_delete.set(None);
                        spawn(async move {
                            match api_client::delete(&format!("/nat/{}", del_id)).await {
                                Ok(_) => {
                                    entries.restart();
                                    notify_change(&mut guard);
                                }
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
                        th { class: TH_CLASS, "Type" }
                        th { class: TH_CLASS, "Source" }
                        th { class: TH_CLASS, "Destination" }
                        th { class: TH_CLASS, "Protocol" }
                        th { class: TH_CLASS, "Translate To" }
                        th { class: TH_CLASS, "Interface" }
                        th { class: TH_CLASS, "Status" }
                        th { class: TH_CLASS, "" }
                    }
                }
                tbody {
                    match &*entries.read() {
                        Some(Ok(list)) if !list.is_empty() => rsx! {
                            for entry in list.iter() {
                                tr { class: TR_CLASS,
                                    key: "{entry.id}",
                                    td { class: TD_CLASS,
                                        Badge {
                                            color: match entry.nat_type {
                                                NatType::SNAT => Color::Blue,
                                                NatType::DNAT => Color::Violet,
                                                NatType::Masquerade => Color::Cyan,
                                            },
                                            label: match entry.nat_type {
                                                NatType::SNAT => "SNAT".to_string(),
                                                NatType::DNAT => "DNAT".to_string(),
                                                NatType::Masquerade => "MASQ".to_string(),
                                            },
                                        }
                                    }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono", {entry.src_network.clone().unwrap_or("*".to_string())} }
                                    td { class: "{TD_CLASS} text-slate-400 font-mono", {format_destination(entry)} }
                                    td { class: "{TD_CLASS} text-slate-400", {entry.protocol.map(|p| format!("{:?}", p)).unwrap_or("Any".to_string())} }
                                    td { class: "{TD_CLASS} text-slate-300 font-mono",
                                        {format_translate(entry)}
                                    }
                                    td { class: "{TD_CLASS} text-slate-400", {format_interface(entry)} }
                                    td { class: TD_CLASS,
                                        Badge {
                                            color: if entry.enabled { Color::Emerald } else { Color::Slate },
                                            label: if entry.enabled { "Enabled".to_string() } else { "Disabled".to_string() },
                                        }
                                    }
                                    td { class: TD_CLASS,
                                        {
                                            let entry_clone = entry.clone();
                                            let id = entry.id;
                                            rsx! {
                                                div { class: "flex items-center gap-1",
                                                    EditBtn {
                                                        onclick: move |_| {
                                                            editing.set(Some((true, entry_clone.clone())));
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
                            TableEmpty { colspan: 8, message: "No NAT entries configured".to_string() }
                        },
                        Some(Err(e)) => rsx! {
                            TableError { colspan: 8, message: format!("Failed to load NAT entries: {e}") }
                        },
                        None => rsx! {
                            TableLoading { colspan: 8 }
                        },
                    }
                }
            }
        }
    }
}

fn format_destination(entry: &NatEntry) -> String {
    let net = entry.dst_network.clone().unwrap_or("*".to_string());
    match &entry.dst_port {
        Some(pr) if pr.start == pr.end => format!("{}:{}", net, pr.start),
        Some(pr) => format!("{}:{}-{}", net, pr.start, pr.end),
        None => net,
    }
}

fn format_interface(entry: &NatEntry) -> String {
    // Show in_interface for DNAT, out_interface for SNAT/Masquerade
    match entry.nat_type {
        NatType::DNAT => entry.in_interface.clone().unwrap_or("\u{2014}".to_string()),
        _ => entry.out_interface.clone().unwrap_or("\u{2014}".to_string()),
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
    let mut ext_port = use_signal(String::new);
    let mut int_ip = use_signal(String::new);
    let mut int_port = use_signal(String::new);
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
                "TCP" => Some(Protocol::TCP),
                "UDP" => Some(Protocol::UDP),
                _ => None, // "TCP + UDP" → None = Any protocol
            },
            dst_port: Some(PortRange::single(ext)),
            in_interface: Some(in_interface()),
            out_interface: None,
            translate_ip: Some(int_ip()),
            translate_port: Some(PortRange::single(internal)),
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
        FormCard { class: "rounded-xl border border-violet-500/20 bg-violet-500/5 p-6 mb-6",
            div { class: "flex items-center gap-2 mb-4",
                Icon { width: 18, height: 18, icon: LdArrowRightLeft, class: "text-violet-400" }
                h3 { class: "text-sm font-semibold text-white", "Port Forward Wizard" }
            }
            p { class: "text-xs text-slate-400 mb-4", "Forward an external port to an internal server. Creates a DNAT rule automatically." }

            if let Some(err) = error() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error.set(None),
                }
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
                FormField { label: "External Port".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", placeholder: "e.g. 8080",
                        value: "{ext_port}",
                        oninput: move |e| ext_port.set(e.value()),
                    }
                }
                FormField { label: "Internal IP".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "e.g. 192.168.1.50",
                        value: "{int_ip}",
                        oninput: move |e| int_ip.set(e.value()),
                    }
                }
                FormField { label: "Internal Port".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "number", placeholder: "Same as external",
                        value: "{int_port}",
                        oninput: move |e| int_port.set(e.value()),
                    }
                }
                FormField { label: "Protocol".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{protocol}", onchange: move |e| protocol.set(e.value()),
                        option { value: "TCP", "TCP" }
                        option { value: "UDP", "UDP" }
                        option { value: "Both", "TCP + UDP" }
                    }
                }
                InterfaceSelect {
                    value: in_interface(),
                    onchange: move |v| in_interface.set(v),
                    label: "WAN Interface",
                }
            }

            SubmitBtn {
                color: Color::Violet,
                label: if submitting() { "Creating...".to_string() } else { "Create Port Forward".to_string() },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}

// === NAT Form (advanced) ===

#[component]
fn NatForm(is_edit: bool, editing: NatEntry, on_saved: EventHandler<()>) -> Element {
    let edit_id = editing.id;
    let mut nat_type = use_signal(|| match editing.nat_type {
        NatType::DNAT => "DNAT".to_string(),
        NatType::Masquerade => "Masquerade".to_string(),
        NatType::SNAT => "SNAT".to_string(),
    });
    let mut src_network = use_signal(|| editing.src_network.clone().unwrap_or_default());
    let mut dst_network = use_signal(|| editing.dst_network.clone().unwrap_or_default());
    let mut translate_ip = use_signal(|| editing.translate_ip.clone().unwrap_or_default());
    let mut out_interface = use_signal(|| editing.out_interface.clone().unwrap_or_default());
    let editing_enabled = editing.enabled;
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let on_submit = move |_| {
        submitting.set(true);
        let entry = NatEntry {
            id: edit_id,
            nat_type: match nat_type().as_str() {
                "DNAT" => NatType::DNAT,
                "Masquerade" => NatType::Masquerade,
                _ => NatType::SNAT,
            },
            enabled: if is_edit { editing_enabled } else { true },
            src_network: if src_network().is_empty() {
                None
            } else {
                Some(src_network())
            },
            dst_network: if dst_network().is_empty() {
                None
            } else {
                Some(dst_network())
            },
            protocol: None,
            dst_port: None,
            in_interface: None,
            out_interface: if out_interface().is_empty() {
                None
            } else {
                Some(out_interface())
            },
            translate_ip: if translate_ip().is_empty() {
                None
            } else {
                Some(translate_ip())
            },
            translate_port: None,
        };
        spawn(async move {
            let result = if is_edit {
                api_client::put::<NatEntry, NatEntry>(&format!("/nat/{}", edit_id), &entry).await
            } else {
                api_client::post::<NatEntry, NatEntry>("/nat", &entry).await
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
                if is_edit { "Edit NAT Entry" } else { "Create NAT Entry" }
            }
            if let Some(err) = error() {
                ErrorAlert {
                    message: err,
                    on_dismiss: move |_| error.set(None),
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-4",
                FormField { label: "NAT Type".to_string(),
                    select {
                        class: SELECT_CLASS,
                        value: "{nat_type}", onchange: move |e| nat_type.set(e.value()),
                        option { value: "SNAT", "SNAT" }
                        option { value: "DNAT", "DNAT" }
                        option { value: "Masquerade", "Masquerade" }
                    }
                }
                FormField { label: "Source Network".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "192.168.1.0/24", value: "{src_network}",
                        oninput: move |e| src_network.set(e.value()),
                    }
                }
                FormField { label: "Destination Network".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "0.0.0.0/0", value: "{dst_network}",
                        oninput: move |e| dst_network.set(e.value()),
                    }
                }
                FormField { label: "Translate IP".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "text", placeholder: "203.0.113.1", value: "{translate_ip}",
                        oninput: move |e| translate_ip.set(e.value()),
                    }
                }
                InterfaceSelect {
                    value: out_interface(),
                    onchange: move |v| out_interface.set(v),
                    label: "Out Interface",
                    allow_empty: true,
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() {
                    if is_edit { "Saving...".to_string() } else { "Creating...".to_string() }
                } else {
                    if is_edit { "Save Entry".to_string() } else { "Create Entry".to_string() }
                },
                disabled: submitting(),
                onclick: on_submit,
            }
        }
    }
}
