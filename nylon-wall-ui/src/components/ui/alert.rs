use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn ErrorAlert(message: String, on_dismiss: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "mb-4 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-400 flex items-center justify-between",
            span { "{message}" }
            button {
                class: "text-red-400 hover:text-red-300",
                onclick: move |e| on_dismiss.call(e),
                Icon { width: 14, height: 14, icon: LdX }
            }
        }
    }
}
