use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

use crate::api_client;
use crate::app::Route;

#[component]
pub fn Setup() -> Element {
    let nav = use_navigator();
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    // If setup is not required (password already set), redirect to login
    use_future(move || async move {
        if let Ok(resp) = api_client::get::<serde_json::Value>("/auth/setup-check").await {
            if !resp.get("setup_required").and_then(|v| v.as_bool()).unwrap_or(true) {
                nav.push(Route::Login);
            }
        }
    });

    let mut on_submit = move |_| {
        if submitting() { return; }
        if password().len() < 8 {
            error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }
        if password() != confirm() {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }
        submitting.set(true);
        error.set(None);
        spawn(async move {
            let body = serde_json::json!({ "password": password() });
            match api_client::post::<serde_json::Value, serde_json::Value>("/auth/setup", &body).await {
                Ok(resp) => {
                    if let Some(token) = resp.get("token").and_then(|t| t.as_str()) {
                        api_client::set_token(token);
                        nav.push(Route::Dashboard);
                    } else {
                        error.set(Some("Invalid response from server".to_string()));
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            submitting.set(false);
        });
    };

    let pw_len = password().len();

    rsx! {
        div { class: "min-h-screen bg-slate-950 flex items-center justify-center",
            div { class: "w-full max-w-sm mx-4",
                // Logo
                div { class: "text-center mb-8",
                    div { class: "inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-gradient-to-br from-blue-500 to-violet-600 shadow-lg shadow-blue-500/20 mb-4",
                        Icon { width: 28, height: 28, icon: LdShield, class: "nw-logo-icon" }
                    }
                    h1 { class: "text-xl font-bold text-white", "Nylon Wall" }
                    p { class: "text-sm text-slate-500 mt-1", "Set up your admin password" }
                }

                // Setup card
                div { class: "rounded-2xl border border-slate-800/60 bg-slate-900/50 p-6",
                    div { class: "mb-4 px-4 py-3 rounded-lg bg-blue-500/10 border border-blue-500/20 text-sm text-blue-400",
                        "Welcome! Create a password to secure your firewall."
                    }

                    if let Some(err) = error() {
                        div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400",
                            "{err}"
                        }
                    }

                    form {
                        onsubmit: move |e| { e.prevent_default(); on_submit(()); },
                        div { class: "mb-4",
                            label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Password" }
                            input {
                                class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/30",
                                r#type: "password",
                                placeholder: "Min. 8 characters",
                                value: "{password}",
                                autofocus: true,
                                oninput: move |e| password.set(e.value()),
                            }
                            if pw_len > 0 && pw_len < 8 {
                                p { class: "text-xs text-amber-400 mt-1",
                                    "{8 - pw_len} more character(s) needed"
                                }
                            }
                        }
                        div { class: "mb-6",
                            label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Confirm Password" }
                            input {
                                class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/30",
                                r#type: "password",
                                placeholder: "Confirm password",
                                value: "{confirm}",
                                oninput: move |e| confirm.set(e.value()),
                            }
                        }
                        button {
                            class: "w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-blue-600 hover:bg-blue-500 text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                            r#type: "submit",
                            disabled: submitting() || pw_len < 8 || confirm().is_empty(),
                            if submitting() { "Setting up..." } else { "Create Password" }
                        }
                    }
                }
            }
        }
    }
}
