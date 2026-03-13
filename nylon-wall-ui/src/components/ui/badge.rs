use dioxus::prelude::*;

use super::Color;

#[component]
pub fn Badge(color: Color, label: String) -> Element {
    rsx! {
        span { class: color.badge_class(), "{label}" }
    }
}
