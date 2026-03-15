use dioxus::prelude::*;
use dioxus::document;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

use crate::api_client;
use crate::app::Route;

#[derive(serde::Deserialize, Clone)]
struct PublicOAuthProvider {
    id: u32,
    name: String,
    provider_type: String,
}

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

    let mut locked_out = use_signal(|| false);

    // Fetch OAuth providers
    let oauth_providers = use_resource(|| async {
        api_client::get::<Vec<PublicOAuthProvider>>("/auth/oauth/providers").await.unwrap_or_default()
    });

    let mut on_submit = move |_| {
        if submitting() { return; }
        submitting.set(true);
        error.set(None);
        spawn(async move {
            let body = serde_json::json!({ "password": password() });
            match api_client::post_with_detail::<serde_json::Value, serde_json::Value>("/auth/login", &body).await {
                Ok(resp) => {
                    if let Some(token) = resp.get("token").and_then(|t| t.as_str()) {
                        api_client::set_token(token);
                        nav.push(Route::Dashboard);
                    } else {
                        error.set(Some("Invalid response from server".to_string()));
                    }
                }
                Err(e) => {
                    if e.starts_with("HTTP 429:") {
                        locked_out.set(true);
                        // Extract remaining seconds from server message
                        let msg = e.strip_prefix("HTTP 429:").unwrap_or(&e);
                        error.set(Some(format!("⏳ {}", msg.trim())));
                    } else if e.contains("UNAUTHORIZED") {
                        locked_out.set(false);
                        error.set(Some("Invalid password".to_string()));
                    } else {
                        locked_out.set(false);
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
                        if locked_out() {
                            div { class: "mb-4 px-4 py-3 rounded-lg bg-amber-500/10 border border-amber-500/20 text-sm text-amber-400",
                                "{err}"
                            }
                        } else {
                            div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400",
                                "{err}"
                            }
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
                            disabled: submitting() || password().is_empty() || locked_out(),
                            if submitting() { "Signing in..." } else { "Sign In" }
                        }
                    }

                    // OAuth provider buttons
                    {
                        let providers = match &*oauth_providers.read() {
                            Some(list) if !list.is_empty() => Some(list.clone()),
                            _ => None,
                        };
                        if let Some(providers) = providers {
                            rsx! {
                                div { class: "mt-5",
                                    div { class: "flex items-center gap-3 mb-4",
                                        div { class: "flex-1 h-px bg-slate-800" }
                                        span { class: "text-[10px] text-slate-600 uppercase tracking-wider", "or" }
                                        div { class: "flex-1 h-px bg-slate-800" }
                                    }
                                    div { class: "space-y-2",
                                        for provider in providers {
                                            {
                                                let pid = provider.id;
                                                let pname = provider.name.clone();
                                                let ptype = provider.provider_type.clone();
                                                rsx! {
                                                    button {
                                                        class: "w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-slate-800/50 border border-slate-700/40 text-slate-300 hover:bg-slate-700/50 hover:text-white transition-colors flex items-center justify-center gap-2",
                                                        r#type: "button",
                                                        onclick: move |_| {
                                                            spawn(async move {
                                                                match api_client::get::<serde_json::Value>(&format!("/auth/oauth/{}/authorize", pid)).await {
                                                                    Ok(resp) => {
                                                                        if let Some(url) = resp.get("url").and_then(|v| v.as_str()) {
                                                                            // Redirect to OAuth provider via JS eval
                                                                            let js = format!("window.location.href = '{}';", url);
                                                                            document::eval(&js);
                                                                        }
                                                                    }
                                                                    Err(e) => error.set(Some(e)),
                                                                }
                                                            });
                                                        },
                                                        if ptype == "github" {
                                                            Icon { width: 16, height: 16, icon: LdGithub }
                                                        } else {
                                                            Icon { width: 16, height: 16, icon: LdLogIn }
                                                        }
                                                        "Sign in with {pname}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
        }
    }
}
