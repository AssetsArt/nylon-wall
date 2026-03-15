use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

use crate::api_client;
use crate::app::Route;

#[component]
pub fn Login() -> Element {
    let nav = use_navigator();
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    // Check if setup is required — redirect to /setup
    use_future(move || async move {
        if let Ok(resp) = api_client::get::<serde_json::Value>("/auth/setup-check").await {
            if resp.get("setup_required").and_then(|v| v.as_bool()).unwrap_or(false) {
                nav.push(Route::Setup);
            }
        }
    });

    let mut on_submit = move |_| {
        if submitting() { return; }
        submitting.set(true);
        error.set(None);
        spawn(async move {
            let body = serde_json::json!({ "password": password() });
            match api_client::post::<serde_json::Value, serde_json::Value>("/auth/login", &body).await {
                Ok(resp) => {
                    if let Some(token) = resp.get("token").and_then(|t| t.as_str()) {
                        api_client::set_token(token);
                        nav.push(Route::Dashboard);
                    } else {
                        error.set(Some("Invalid response from server".to_string()));
                    }
                }
                Err(e) => {
                    if e.contains("401") || e.contains("UNAUTHORIZED") {
                        error.set(Some("Invalid password".to_string()));
                    } else {
                        error.set(Some(e));
                    }
                }
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen bg-slate-950 flex items-center justify-center",
            div { class: "w-full max-w-sm mx-4",
                // Logo
                div { class: "text-center mb-8",
                    div { class: "inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-gradient-to-br from-blue-500 to-violet-600 shadow-lg shadow-blue-500/20 mb-4",
                        Icon { width: 28, height: 28, icon: LdShield, class: "nw-logo-icon" }
                    }
                    h1 { class: "text-xl font-bold text-white", "Nylon Wall" }
                    p { class: "text-sm text-slate-500 mt-1", "Sign in to your firewall" }
                }

                // Login card
                div { class: "rounded-2xl border border-slate-800/60 bg-slate-900/50 p-6",
                    if let Some(err) = error() {
                        div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400",
                            "{err}"
                        }
                    }

                    form {
                        onsubmit: move |e| { e.prevent_default(); on_submit(()); },
                        div { class: "mb-4",
                            label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Username" }
                            input {
                                class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-slate-400 cursor-not-allowed",
                                r#type: "text",
                                value: "admin",
                                disabled: true,
                            }
                        }
                        div { class: "mb-6",
                            label { class: "block text-xs font-medium text-slate-400 mb-1.5", "Password" }
                            input {
                                class: "w-full px-3 py-2 rounded-lg bg-slate-800/50 border border-slate-700/40 text-sm text-white placeholder-slate-600 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/30",
                                r#type: "password",
                                placeholder: "Enter password",
                                value: "{password}",
                                autofocus: true,
                                oninput: move |e| password.set(e.value()),
                            }
                        }
                        button {
                            class: "w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-blue-600 hover:bg-blue-500 text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                            r#type: "submit",
                            disabled: submitting() || password().is_empty(),
                            if submitting() { "Signing in..." } else { "Sign In" }
                        }
                    }
                }
            }
        }
    }
}
