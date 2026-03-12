use dioxus::prelude::*;

use crate::components::*;

#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/")]
        Dashboard,
        #[route("/rules")]
        Rules,
        #[route("/nat")]
        Nat,
        #[route("/routes")]
        Routes,
        #[route("/policies")]
        Policies,
        #[route("/logs")]
        Logs,
        #[route("/settings")]
        Settings,
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Layout() -> Element {
    rsx! {
        div { class: "app-layout",
            nav { class: "sidebar",
                div { class: "sidebar-header",
                    h2 { "Nylon Wall" }
                }
                div { class: "sidebar-nav",
                    Link { to: Route::Dashboard, class: "nav-item", "Dashboard" }
                    Link { to: Route::Rules, class: "nav-item", "Firewall Rules" }
                    Link { to: Route::Nat, class: "nav-item", "NAT" }
                    Link { to: Route::Routes, class: "nav-item", "Routes" }
                    Link { to: Route::Policies, class: "nav-item", "Policies" }
                    Link { to: Route::Logs, class: "nav-item", "Logs" }
                    Link { to: Route::Settings, class: "nav-item", "Settings" }
                }
            }
            main { class: "main-content",
                Outlet::<Route> {}
            }
        }
    }
}
