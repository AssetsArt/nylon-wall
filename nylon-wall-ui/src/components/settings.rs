use super::ui::*;
use super::{ConfirmModal, notify_change, use_change_guard, use_refresh_trigger};
use crate::api_client;
use crate::models::*;
use crate::ws_client::use_ws_events;
use dioxus::document;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[derive(Debug, Clone, serde::Deserialize)]
struct NetworkInterface {
    name: String,
    mac: String,
    ip: String,
    status: String,
    mtu: u32,
}

#[component]
pub fn Settings() -> Element {
    let mut status = use_context::<Resource<Result<SystemStatus, String>>>();
    let mut interfaces = use_resource(|| async {
        api_client::get::<Vec<NetworkInterface>>("/system/interfaces").await
    });
    let mut guard = use_change_guard();
    let mut backup_msg = use_signal(|| None::<(bool, String)>);
    let mut importing = use_signal(|| false);
    let mut confirm_import = use_signal(|| None::<String>);

    // Live uptime counter — syncs from API, then ticks locally every second
    let mut uptime = use_signal(|| 0u64);
    use_effect(move || {
        if let Some(Ok(s)) = &*status.read() {
            uptime.set(s.uptime_seconds);
        }
    });
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(1_000).await;
            uptime += 1;
        }
    });

    let ws = use_ws_events();
    let refresh = use_refresh_trigger();
    let mut prev = use_signal(|| (refresh(), ws.system()));
    use_effect(move || {
        let current = (refresh(), ws.system());
        if current != prev() {
            prev.set(current);
            status.restart();
            interfaces.restart();
        }
    });

    rsx! {
        div {
            PageHeader {
                title: "Settings".to_string(),
                subtitle: "System configuration and maintenance".to_string(),
            }

            if let Some((success, msg)) = backup_msg() {
                div {
                    class: if success {
                        "mb-4 px-4 py-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-sm text-emerald-400 flex items-center justify-between"
                    } else {
                        "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 flex items-center justify-between"
                    },
                    span { "{msg}" }
                    button {
                        class: "text-slate-400 hover:text-slate-300",
                        onclick: move |_| backup_msg.set(None),
                        Icon { width: 14, height: 14, icon: LdX }
                    }
                }
            }

            if confirm_import().is_some() {
                ConfirmModal {
                    title: "Import Configuration".to_string(),
                    message: "This will replace all current rules, NAT entries, routes, zones, and policies with the imported configuration. This action cannot be undone.".to_string(),
                    confirm_label: "Import".to_string(),
                    danger: false,
                    on_confirm: move |_| {
                        if let Some(content) = confirm_import() {
                            confirm_import.set(None);
                            spawn(async move {
                                match serde_json::from_str::<serde_json::Value>(&content) {
                                    Ok(backup_data) => {
                                        match api_client::post::<serde_json::Value, serde_json::Value>("/system/restore", &backup_data).await {
                                            Ok(resp) => {
                                                let status = resp.get("status").and_then(|s| s.as_str()).unwrap_or("done");
                                                backup_msg.set(Some((true, format!("Configuration restored ({})", status))));
                                                notify_change(&mut guard);
                                            }
                                            Err(e) => backup_msg.set(Some((false, format!("Restore failed: {}", e)))),
                                        }
                                    }
                                    Err(e) => backup_msg.set(Some((false, format!("Invalid backup file: {}", e)))),
                                }
                            });
                        }
                    },
                    on_cancel: move |_| { confirm_import.set(None); },
                }
            }

            // System Information
            div { class: "mb-6",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdServer, class: "text-slate-500" } },
                    title: "System Information".to_string(),
                }
                FormCard { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-0",
                    match &*status.read() {
                        Some(Ok(s)) => rsx! {
                            div { class: "space-y-3",
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Version" }
                                    span { class: "text-sm text-slate-300 font-mono", "v{s.version}" }
                                }
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "eBPF Status" }
                                    Badge {
                                        color: if s.ebpf_loaded { Color::Emerald } else { Color::Red },
                                        label: if s.ebpf_loaded { "Loaded".to_string() } else { "Not Loaded".to_string() },
                                    }
                                }
                                div { class: "flex items-center justify-between py-2 border-b border-slate-800/40",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Uptime" }
                                    span { class: "text-sm text-slate-300 font-mono",
                                        {format_uptime(uptime())}
                                    }
                                }
                                div { class: "flex items-center justify-between py-2",
                                    span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "Engine" }
                                    span { class: "text-sm text-slate-300", "eBPF / XDP" }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "flex items-center gap-2 text-red-400",
                                Icon { width: 14, height: 14, icon: LdTriangleAlert }
                                span { class: "text-sm", "Failed to load status: {e}" }
                            }
                        },
                        None => rsx! {
                            p { class: "text-sm text-slate-600", "Loading..." }
                        },
                    }
                }
            }

            // eBPF Programs
            match &*status.read() {
                Some(Ok(s)) if s.ebpf_loaded && !s.ebpf_programs.is_empty() => rsx! {
                    div { class: "mb-6",
                        SectionHeader {
                            icon: rsx! { Icon { width: 15, height: 15, icon: LdCpu, class: "text-slate-500" } },
                            title: "eBPF Programs".to_string(),
                        }
                        DataTable {
                            thead { class: "bg-slate-900/80",
                                tr {
                                    th { class: TH_CLASS, "Program" }
                                    th { class: TH_CLASS, "Type" }
                                    th { class: TH_CLASS, "Role" }
                                    th { class: TH_CLASS, "Stage" }
                                    th { class: TH_CLASS, "Status" }
                                }
                            }
                            tbody {
                                for prog in s.ebpf_programs.iter() {
                                    tr { class: TR_CLASS,
                                        key: "{prog.name}",
                                        td { class: "{TD_CLASS} text-slate-300 font-mono text-xs", "{prog.name}" }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if prog.prog_type == "XDP" { Color::Cyan } else { Color::Violet },
                                                label: prog.prog_type.clone(),
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: if prog.role == "entry" { Color::Blue } else { Color::Slate },
                                                label: prog.role.clone(),
                                            }
                                        }
                                        td { class: "{TD_CLASS} text-slate-500 font-mono text-xs",
                                            match prog.stage {
                                                Some(0) => rsx! { "NAT" },
                                                Some(1) => rsx! { "SNI" },
                                                Some(2) => rsx! { "Rules" },
                                                Some(n) => rsx! { "{n}" },
                                                None => rsx! { span { class: "text-slate-700", "\u{2014}" } },
                                            }
                                        }
                                        td { class: TD_CLASS,
                                            Badge {
                                                color: Color::Emerald,
                                                label: "Loaded".to_string(),
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

            // Network Interfaces (only those with a non-empty status)
            div { class: "mb-6",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdNetwork, class: "text-slate-500" } },
                    title: "Network Interfaces".to_string(),
                }
                DataTable {
                    thead { class: "bg-slate-900/80",
                        tr {
                            th { class: TH_CLASS, "Name" }
                            th { class: TH_CLASS, "IP Address" }
                            th { class: TH_CLASS, "MAC" }
                            th { class: TH_CLASS, "MTU" }
                            th { class: TH_CLASS, "Status" }
                            th { class: TH_CLASS, "Zone" }
                        }
                    }
                    tbody {
                        match &*interfaces.read() {
                            Some(Ok(list)) => {
                                let active: Vec<_> = list.iter().filter(|i| !i.status.is_empty() && i.status != "unknown").collect();
                                if active.is_empty() {
                                    rsx! {
                                        TableEmpty { colspan: 6, message: "No active interfaces found".to_string() }
                                    }
                                } else {
                                    rsx! {
                                        for iface in active.iter() {
                                            tr { class: TR_CLASS,
                                                key: "{iface.name}",
                                                td { class: "{TD_CLASS} text-slate-300 font-mono font-medium", "{iface.name}" }
                                                td { class: "{TD_CLASS} text-slate-400 font-mono", "{iface.ip}" }
                                                td { class: "{TD_CLASS} text-slate-500 font-mono", "{iface.mac}" }
                                                td { class: "{TD_CLASS} text-slate-500 font-mono", "{iface.mtu}" }
                                                td { class: TD_CLASS,
                                                    Badge {
                                                        color: if iface.status == "up" { Color::Emerald } else { Color::Slate },
                                                        label: iface.status.clone(),
                                                    }
                                                }
                                                td { class: TD_CLASS,
                                                    {
                                                        let zone = match iface.name.as_str() {
                                                            "eth0" => "WAN",
                                                            "eth1" => "LAN",
                                                            "lo" => "Local",
                                                            _ => "Unassigned",
                                                        };
                                                        rsx! {
                                                            Badge { color: Color::Blue, label: zone.to_string() }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Err(e)) => rsx! {
                                TableError { colspan: 6, message: format!("Failed to load: {e}") }
                            },
                            None => rsx! {
                                TableLoading { colspan: 6 }
                            },
                        }
                    }
                }
            }

            // Change Password
            div {
                class: "mb-6",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdHardDrive, class: "text-slate-500" } },
                    title: "Change Password".to_string(),
                }
                FormCard {
                    ChangePasswordForm {}
                }
            }

            // OAuth Providers
            div {
                class: "mb-6",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdKeyRound, class: "text-violet-400" } },
                    title: "OAuth / SSO Providers".to_string(),
                }
                OAuthProviderSection {}
            }

            // Backup & Restore
            div {
                class: "mb-6",
                SectionHeader {
                    icon: rsx! { Icon { width: 15, height: 15, icon: LdHardDrive, class: "text-slate-500" } },
                    title: "Backup & Restore".to_string(),
                }
                FormCard { class: "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-0",
                    p { class: "text-sm text-slate-400 mb-4", "Export or import your firewall configuration for backup or migration." }
                    div { class: "flex items-center gap-3",
                        Btn {
                            color: Color::Blue,
                            label: "Export Configuration".to_string(),
                            icon: rsx! { Icon { width: 13, height: 13, icon: LdDownload } },
                            onclick: move |_| {
                                spawn(async move {
                                    match api_client::post::<(), serde_json::Value>("/system/backup", &()).await {
                                        Ok(data) => {
                                            let json_str = serde_json::to_string_pretty(&data).unwrap_or_default();
                                            let bytes_len = json_str.len();
                                            // Use JS to trigger a file download via Blob
                                            let js_code = format!(
                                                r#"
                                                (function() {{
                                                    var json = {};
                                                    var blob = new Blob([JSON.stringify(json, null, 2)], {{ type: 'application/json' }});
                                                    var url = URL.createObjectURL(blob);
                                                    var a = document.createElement('a');
                                                    a.href = url;
                                                    a.download = 'nylon-wall-backup.json';
                                                    document.body.appendChild(a);
                                                    a.click();
                                                    document.body.removeChild(a);
                                                    URL.revokeObjectURL(url);
                                                }})();
                                                "#,
                                                json_str
                                            );
                                            document::eval(&js_code);
                                            backup_msg.set(Some((true, format!("Backup exported ({} bytes)", bytes_len))));
                                        }
                                        Err(e) => backup_msg.set(Some((false, format!("Backup failed: {}", e)))),
                                    }
                                });
                            },
                        }
                        Btn {
                            color: Color::Slate,
                            label: if importing() { "Importing...".to_string() } else { "Import Configuration".to_string() },
                            disabled: importing(),
                            icon: rsx! { Icon { width: 13, height: 13, icon: LdUpload } },
                            onclick: move |_| {
                                importing.set(true);
                                spawn(async move {
                                    // Create a file input, trigger click, read file content
                                    // Uses window focus event to detect cancel (file dialog closing without selection)
                                    let js_code = r#"
                                        var input = document.createElement('input');
                                        input.type = 'file';
                                        input.accept = '.json';
                                        var handled = false;
                                        input.onchange = function(e) {
                                            handled = true;
                                            var file = e.target.files[0];
                                            if (!file) { dioxus.send(''); return; }
                                            var reader = new FileReader();
                                            reader.onload = function(ev) { dioxus.send(ev.target.result); };
                                            reader.onerror = function() { dioxus.send(''); };
                                            reader.readAsText(file);
                                        };
                                        window.addEventListener('focus', function onFocus() {
                                            window.removeEventListener('focus', onFocus);
                                            setTimeout(function() {
                                                if (!handled) { dioxus.send(''); }
                                            }, 500);
                                        });
                                        input.click();
                                    "#;
                                    let mut eval = document::eval(js_code);
                                    match eval.recv::<String>().await {
                                        Ok(file_content) => {
                                            if file_content.is_empty() {
                                                importing.set(false);
                                                return;
                                            }
                                            // Validate JSON before showing confirm
                                            match serde_json::from_str::<serde_json::Value>(&file_content) {
                                                Ok(_) => {
                                                    // Store content and show confirm modal
                                                    confirm_import.set(Some(file_content));
                                                }
                                                Err(e) => {
                                                    backup_msg.set(Some((false, format!("Invalid backup file: {}", e))));
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            backup_msg.set(Some((false, "File read cancelled or failed".to_string())));
                                        }
                                    }
                                    importing.set(false);
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ChangePasswordForm() -> Element {
    let mut current = use_signal(String::new);
    let mut new_pw = use_signal(String::new);
    let mut confirm_pw = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut submitting = use_signal(|| false);

    let mut do_submit = move || {
        if submitting() {
            return;
        }
        if new_pw().len() < 8 {
            error.set(Some(
                "New password must be at least 8 characters".to_string(),
            ));
            return;
        }
        if new_pw() != confirm_pw() {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }
        submitting.set(true);
        error.set(None);
        success.set(false);
        spawn(async move {
            let body = serde_json::json!({
                "current_password": current(),
                "new_password": new_pw(),
            });
            match api_client::put::<serde_json::Value, serde_json::Value>("/auth/password", &body)
                .await
            {
                Ok(_) => {
                    success.set(true);
                    current.set(String::new());
                    new_pw.set(String::new());
                    confirm_pw.set(String::new());
                }
                Err(e) => {
                    if e.contains("401") || e.contains("UNAUTHORIZED") {
                        error.set(Some("Current password is incorrect".to_string()));
                    } else {
                        error.set(Some(e));
                    }
                }
            }
            submitting.set(false);
        });
    };

    rsx! {
        if success() {
            div { class: "mb-4 px-4 py-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-sm text-emerald-400",
                "Password changed successfully"
            }
        }
        if let Some(err) = error() {
            div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400",
                "{err}"
            }
        }
        form {
            onsubmit: move |e| { e.prevent_default(); do_submit(); },
            div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4 mb-4",
                FormField { label: "Current Password".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "password",
                        placeholder: "Current password",
                        value: "{current}",
                        oninput: move |e| current.set(e.value()),
                    }
                }
                FormField { label: "New Password".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "password",
                        placeholder: "Min. 8 characters",
                        value: "{new_pw}",
                        oninput: move |e| new_pw.set(e.value()),
                    }
                }
                FormField { label: "Confirm New Password".to_string(),
                    input {
                        class: INPUT_CLASS,
                        r#type: "password",
                        placeholder: "Confirm password",
                        value: "{confirm_pw}",
                        oninput: move |e| confirm_pw.set(e.value()),
                    }
                }
            }
            SubmitBtn {
                color: Color::Blue,
                label: if submitting() { "Changing...".to_string() } else { "Change Password".to_string() },
                disabled: submitting() || current().is_empty() || new_pw().len() < 8 || confirm_pw().is_empty(),
                onclick: move |_| do_submit(),
            }
        }
    }
}

#[component]
fn OAuthProviderSection() -> Element {
    let mut providers = use_resource(|| async {
        api_client::get::<Vec<OAuthProvider>>("/auth/oauth/manage").await
    });

    let mut show_form = use_signal(|| false);
    let mut editing = use_signal(|| None::<OAuthProvider>);
    let mut confirm_delete = use_signal(|| None::<u32>);
    let mut error_msg = use_signal(|| None::<String>);

    rsx! {
        if let Some(err) = error_msg() {
            ErrorAlert {
                message: err,
                on_dismiss: move |_| error_msg.set(None),
            }
        }

        FormCard {
            div { class: "flex items-center justify-between mb-4",
                div {
                    p { class: "text-sm text-slate-400",
                        "Configure OAuth/OIDC providers for single sign-on."
                    }
                }
                Btn {
                    color: Color::Violet,
                    label: if show_form() { "Cancel".to_string() } else { "+ Add Provider".to_string() },
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

            if show_form() {
                OAuthProviderForm {
                    provider: editing(),
                    on_saved: move |_| {
                        show_form.set(false);
                        editing.set(None);
                        providers.restart();
                    },
                    on_cancel: move |_| {
                        show_form.set(false);
                        editing.set(None);
                    },
                    error_msg: error_msg,
                }
            }

            match &*providers.read() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    EmptyState {
                        icon: rsx! { Icon { width: 32, height: 32, icon: LdKeyRound } },
                        title: "No OAuth Providers".to_string(),
                        subtitle: "Add Google, GitHub, or a custom OIDC provider for SSO login.".to_string(),
                    }
                },
                Some(Ok(list)) => rsx! {
                    div { class: "space-y-3",
                        for provider in list {
                            {
                                let p = provider.clone();
                                let p_edit = provider.clone();
                                let pid = provider.id;
                                rsx! {
                                    div { class: "flex items-center justify-between rounded-lg border border-slate-800/60 bg-slate-950/30 px-4 py-3",
                                        div { class: "flex items-center gap-3",
                                            div { class: "w-8 h-8 rounded-lg bg-violet-500/10 flex items-center justify-center",
                                                match p.provider_type {
                                                    OAuthProviderType::GitHub => rsx! { Icon { width: 16, height: 16, icon: LdGithub, class: "text-violet-400" } },
                                                    _ => rsx! { Icon { width: 16, height: 16, icon: LdLogIn, class: "text-violet-400" } },
                                                }
                                            }
                                            div {
                                                p { class: "text-sm font-medium text-white", "{p.name}" }
                                                p { class: "text-[10px] text-slate-600",
                                                    "{p.provider_type.label()} — Client ID: {p.client_id}"
                                                }
                                            }
                                        }
                                        div { class: "flex items-center gap-2",
                                            Badge {
                                                color: if p.enabled { Color::Emerald } else { Color::Slate },
                                                label: if p.enabled { "Enabled".to_string() } else { "Disabled".to_string() },
                                            }
                                            button {
                                                class: "w-7 h-7 rounded-lg hover:bg-violet-500/10 flex items-center justify-center transition-colors",
                                                title: if p.enabled { "Disable" } else { "Enable" },
                                                onclick: move |_| {
                                                    spawn(async move {
                                                        let _ = api_client::post::<(), serde_json::Value>(&format!("/auth/oauth/manage/{}/toggle", pid), &()).await;
                                                        providers.restart();
                                                    });
                                                },
                                                if p.enabled {
                                                    Icon { width: 12, height: 12, icon: LdToggleRight, class: "text-emerald-400" }
                                                } else {
                                                    Icon { width: 12, height: 12, icon: LdToggleLeft, class: "text-slate-500" }
                                                }
                                            }
                                            button {
                                                class: "w-7 h-7 rounded-lg hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                                                title: "Edit",
                                                onclick: move |_| {
                                                    editing.set(Some(p_edit.clone()));
                                                    show_form.set(true);
                                                },
                                                Icon { width: 12, height: 12, icon: LdPencil, class: "text-slate-400" }
                                            }
                                            button {
                                                class: "w-7 h-7 rounded-lg hover:bg-red-500/10 flex items-center justify-center transition-colors",
                                                title: "Delete",
                                                onclick: move |_| confirm_delete.set(Some(pid)),
                                                Icon { width: 12, height: 12, icon: LdTrash2, class: "text-red-400" }
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
        }

        // Delete confirmation
        if let Some(id) = confirm_delete() {
            ConfirmModal {
                title: "Delete Provider".to_string(),
                message: "Remove this OAuth provider? Users will no longer be able to sign in with it.".to_string(),
                on_confirm: move |_| {
                    spawn(async move {
                        let _ = api_client::delete(&format!("/auth/oauth/manage/{}", id)).await;
                        confirm_delete.set(None);
                        providers.restart();
                    });
                },
                on_cancel: move |_| confirm_delete.set(None),
            }
        }
    }
}

#[component]
fn OAuthProviderForm(
    provider: Option<OAuthProvider>,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
    mut error_msg: Signal<Option<String>>,
) -> Element {
    let is_edit = provider.is_some();
    let initial = provider.unwrap_or(OAuthProvider {
        id: 0,
        provider_type: OAuthProviderType::Google,
        name: String::new(),
        client_id: String::new(),
        client_secret: String::new(),
        enabled: true,
        issuer_url: String::new(),
        authorize_url: String::new(),
        token_url: String::new(),
        userinfo_url: String::new(),
        scopes: Vec::new(),
    });

    let mut provider_type = use_signal(move || initial.provider_type.clone());
    let mut name = use_signal(move || initial.name.clone());
    let mut client_id = use_signal(move || initial.client_id.clone());
    let mut client_secret = use_signal(move || initial.client_secret.clone());
    let mut issuer_url = use_signal(move || initial.issuer_url.clone());
    let mut submitting = use_signal(|| false);
    let entry_id = initial.id;

    // Auto-fill name from provider type
    let mut update_name = move |pt: &OAuthProviderType| {
        if name().is_empty() || name() == "Google" || name() == "GitHub" || name() == "Custom OIDC" {
            name.set(pt.label().to_string());
        }
    };

    rsx! {
        div { class: "rounded-lg border border-violet-500/20 bg-violet-500/5 p-4 mb-4",
            h4 { class: "text-sm font-semibold text-white mb-3",
                if is_edit { "Edit Provider" } else { "New OAuth Provider" }
            }

            div { class: "grid grid-cols-2 gap-4 mb-4",
                div {
                    label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Provider Type" }
                    select {
                        class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white",
                        value: match provider_type() {
                            OAuthProviderType::Google => "google",
                            OAuthProviderType::GitHub => "github",
                            OAuthProviderType::Oidc => "oidc",
                        },
                        onchange: move |e| {
                            let pt = match e.value().as_str() {
                                "github" => OAuthProviderType::GitHub,
                                "oidc" => OAuthProviderType::Oidc,
                                _ => OAuthProviderType::Google,
                            };
                            update_name(&pt);
                            provider_type.set(pt);
                        },
                        option { value: "google", "Google" }
                        option { value: "github", "GitHub" }
                        option { value: "oidc", "Custom OIDC" }
                    }
                }
                div {
                    label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Display Name" }
                    input {
                        class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600",
                        r#type: "text",
                        placeholder: "Google",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
            }

            div { class: "grid grid-cols-2 gap-4 mb-4",
                div {
                    label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Client ID" }
                    input {
                        class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 font-mono",
                        r#type: "text",
                        placeholder: "your-client-id",
                        value: "{client_id}",
                        oninput: move |e| client_id.set(e.value()),
                    }
                }
                div {
                    label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Client Secret" }
                    input {
                        class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 font-mono",
                        r#type: "password",
                        placeholder: if is_edit { "••••••••" } else { "your-client-secret" },
                        value: "{client_secret}",
                        oninput: move |e| client_secret.set(e.value()),
                    }
                }
            }

            // Custom OIDC fields
            if provider_type() == OAuthProviderType::Oidc {
                div { class: "mb-4",
                    label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Issuer URL / Discovery URL" }
                    input {
                        class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 font-mono",
                        r#type: "url",
                        placeholder: "https://accounts.example.com/.well-known/openid-configuration",
                        value: "{issuer_url}",
                        oninput: move |e| issuer_url.set(e.value()),
                    }
                }
            }

            div { class: "flex gap-2",
                Btn {
                    color: Color::Violet,
                    label: if submitting() { "Saving...".to_string() } else if is_edit { "Update".to_string() } else { "Add Provider".to_string() },
                    disabled: submitting() || client_id().is_empty() || name().is_empty(),
                    onclick: move |_| {
                        submitting.set(true);
                        error_msg.set(None);
                        let body = OAuthProvider {
                            id: entry_id,
                            provider_type: provider_type(),
                            name: name(),
                            client_id: client_id(),
                            client_secret: client_secret(),
                            enabled: true,
                            issuer_url: issuer_url(),
                            authorize_url: String::new(),
                            token_url: String::new(),
                            userinfo_url: String::new(),
                            scopes: Vec::new(),
                        };
                        spawn(async move {
                            let result = if is_edit {
                                api_client::put::<OAuthProvider, OAuthProvider>(&format!("/auth/oauth/manage/{}", entry_id), &body).await
                            } else {
                                api_client::post::<OAuthProvider, OAuthProvider>("/auth/oauth/manage", &body).await
                            };
                            match result {
                                Ok(_) => on_saved.call(()),
                                Err(e) => error_msg.set(Some(e)),
                            }
                            submitting.set(false);
                        });
                    },
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

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, mins, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
