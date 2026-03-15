use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

use crate::components::*;
use crate::components::change_guard;
use crate::models::SystemStatus;
use crate::theme::{self, Theme};
use crate::api_client;
use crate::ws_client;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login")]
    Login,
    #[route("/setup")]
    Setup,
    #[layout(Layout)]
        #[route("/")]
        Dashboard,
        #[route("/rules")]
        Rules,
        #[route("/nat")]
        Nat,
        #[route("/routes")]
        Routes,
        #[route("/dhcp")]
        Dhcp,
        #[route("/policies")]
        Policies,
        #[route("/tls")]
        Tls,
        #[route("/connections")]
        Connections,
        #[route("/logs")]
        Logs,
        #[route("/settings")]
        Settings,
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Layout() -> Element {
    let route: Route = use_route();
    let nav = use_navigator();
    let theme = theme::use_theme_init();
    let _change_guard = change_guard::use_change_guard_provider();
    ws_client::use_ws_provider();

    // Auth guard — check token + setup status on mount
    let mut auth_checked = use_signal(|| false);
    use_future(move || async move {
        // Check if setup is required first
        if let Ok(resp) = api_client::get::<serde_json::Value>("/auth/setup-check").await {
            if resp.get("setup_required").and_then(|v| v.as_bool()).unwrap_or(false) {
                nav.push(Route::Setup);
                return;
            }
        }
        // Check if we have a valid token
        if !api_client::has_token() {
            nav.push(Route::Login);
            return;
        }
        // Validate token with server
        match api_client::get::<serde_json::Value>("/auth/check").await {
            Ok(_) => { auth_checked.set(true); }
            Err(e) => {
                if e == api_client::UNAUTHORIZED {
                    nav.push(Route::Login);
                } else {
                    // Server unreachable — allow through (offline mode)
                    auth_checked.set(true);
                }
            }
        }
    });

    // Provide SystemStatus as context — shared by Dashboard, Settings, and sidebar
    let status = use_resource(|| async { api_client::get::<SystemStatus>("/system/status").await });
    use_context_provider(|| status);

    let nav_cls = |target: &Route| {
        if *target == route {
            "flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium text-blue-400 bg-blue-500/10 ring-1 ring-blue-500/20"
        } else {
            "flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-all"
        }
    };
    let sub_cls = |target: &Route| {
        if *target == route {
            "flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-[13px] font-medium text-blue-400 bg-blue-500/10 ring-1 ring-blue-500/20"
        } else {
            "flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-[13px] font-medium text-slate-500 hover:text-slate-300 hover:bg-white/5 transition-all"
        }
    };

    let on_logout = move |_| {
        spawn(async move {
            let _ = api_client::post::<(), serde_json::Value>("/auth/logout", &()).await;
            api_client::clear_token();
            nav.push(Route::Login);
        });
    };

    rsx! {
        div { class: "flex min-h-screen bg-slate-950",
            // Sidebar
            nav { class: "w-56 bg-slate-950 border-r border-slate-800/60 fixed top-0 left-0 bottom-0 flex flex-col",
                // Brand
                div { class: "px-4 py-5 border-b border-slate-800/60",
                    div { class: "flex items-center gap-2.5",
                        div { class: "w-8 h-8 rounded-xl bg-gradient-to-br from-blue-500 to-violet-600 flex items-center justify-center shrink-0 shadow-lg shadow-blue-500/20",
                            Icon { width: 20, height: 20, icon: LdShield, class: "nw-logo-icon" }
                        }
                        div {
                            p { class: "text-sm font-bold text-white leading-tight", "Nylon Wall" }
                            p { class: "text-[9px] font-medium text-slate-500 uppercase tracking-widest", "firewall" }
                        }
                    }
                }

                // Navigation
                div { class: "flex-1 px-2 py-3 overflow-y-auto space-y-0.5",
                    p { class: "text-[9px] font-bold uppercase tracking-[0.15em] text-slate-600 px-3 py-1 mt-0.5",
                        "Overview"
                    }
                    Link { class: nav_cls(&Route::Dashboard), to: Route::Dashboard,
                        Icon { width: 14, height: 14, icon: LdLayoutDashboard }
                        "Dashboard"
                    }

                    p { class: "text-[9px] font-bold uppercase tracking-[0.15em] text-slate-600 px-3 py-1 mt-3",
                        "Firewall"
                    }
                    Link { class: sub_cls(&Route::Rules), to: Route::Rules,
                        Icon { width: 13, height: 13, icon: LdShieldCheck }
                        "Rules"
                    }
                    Link { class: sub_cls(&Route::Nat), to: Route::Nat,
                        Icon { width: 13, height: 13, icon: LdArrowLeftRight }
                        "NAT"
                    }
                    Link { class: sub_cls(&Route::Tls), to: Route::Tls,
                        Icon { width: 13, height: 13, icon: LdLock }
                        "TLS / SNI"
                    }

                    p { class: "text-[9px] font-bold uppercase tracking-[0.15em] text-slate-600 px-3 py-1 mt-3",
                        "Network"
                    }
                    Link { class: sub_cls(&Route::Routes), to: Route::Routes,
                        Icon { width: 13, height: 13, icon: LdNetwork }
                        "Routes"
                    }
                    Link { class: sub_cls(&Route::Dhcp), to: Route::Dhcp,
                        Icon { width: 13, height: 13, icon: LdWifi }
                        "DHCP"
                    }
                    Link { class: sub_cls(&Route::Policies), to: Route::Policies,
                        Icon { width: 13, height: 13, icon: LdLayers }
                        "Policies"
                    }

                    p { class: "text-[9px] font-bold uppercase tracking-[0.15em] text-slate-600 px-3 py-1 mt-3",
                        "Monitor"
                    }
                    Link { class: sub_cls(&Route::Connections), to: Route::Connections,
                        Icon { width: 13, height: 13, icon: LdCable }
                        "Connections"
                    }
                    Link { class: sub_cls(&Route::Logs), to: Route::Logs,
                        Icon { width: 13, height: 13, icon: LdScroll }
                        "Logs"
                    }

                    p { class: "text-[9px] font-bold uppercase tracking-[0.15em] text-slate-600 px-3 py-1 mt-3",
                        "System"
                    }
                    Link { class: sub_cls(&Route::Settings), to: Route::Settings,
                        Icon { width: 13, height: 13, icon: LdSettings }
                        "Settings"
                    }
                }

                // Footer
                div { class: "px-4 py-3 border-t border-slate-800/60 flex items-center justify-between",
                    p { class: "text-[10px] text-slate-700 font-mono",
                        {match &*status.read() {
                            Some(Ok(s)) => format!("v{}", s.version),
                            _ => "v...".to_string(),
                        }}
                    }
                    div { class: "flex items-center gap-1",
                        button {
                            class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                            title: "Logout",
                            onclick: on_logout,
                            Icon { width: 14, height: 14, icon: LdLogOut, class: "text-slate-400" }
                        }
                        button {
                            class: "w-7 h-7 rounded-lg bg-slate-800/50 hover:bg-slate-700/50 flex items-center justify-center transition-colors",
                            title: if *theme.read() == Theme::Dark { "Switch to light mode" } else { "Switch to dark mode" },
                            onclick: move |_| { theme::toggle_theme(theme); },
                            if *theme.read() == Theme::Dark {
                                Icon { width: 14, height: 14, icon: LdSun, class: "text-slate-400" }
                            } else {
                                Icon { width: 14, height: 14, icon: LdMoon, class: "text-slate-400" }
                            }
                        }
                    }
                }
            }

            // Main content
            main { class: "ml-56 flex-1 min-h-screen",
                div { class: "max-w-screen-xl mx-auto px-8 py-8",
                    Outlet::<Route> {}
                }
            }

            // Change confirmation modal (auto-revert countdown)
            ChangeTimerModal {}
        }
    }
}
