use dioxus::prelude::*;

use super::Color;

#[component]
pub fn StatCard(
    color: Color,
    icon: Element,
    label: String,
    value: String,
    #[props(default)] subtitle: Option<String>,
) -> Element {
    rsx! {
        div { class: color.stat_card_class(),
            div { class: "flex items-center gap-3 mb-3",
                div { class: color.icon_bg_class(), {icon} }
                span { class: "text-xs font-medium text-slate-500 uppercase tracking-wider", "{label}" }
            }
            p { class: "text-2xl font-bold text-white mb-1", "{value}" }
            if let Some(sub) = subtitle {
                p { class: "text-xs text-slate-500", "{sub}" }
            }
        }
    }
}

#[component]
pub fn FormCard(
    #[props(default = "rounded-xl border border-slate-800/60 bg-slate-900/50 p-6 mb-6")]
    class: &'static str,
    children: Element,
) -> Element {
    rsx! {
        div { class: class, {children} }
    }
}

#[component]
pub fn EmptyState(
    icon: Element,
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] children: Element,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-dashed border-slate-800/60 p-12 text-center",
            style: "text-align: -webkit-center;",
            div { class: "flex justify-center mb-4",
                div { class: "w-12 h-12 rounded-xl bg-slate-800/50 flex items-center justify-center",
                    {icon}
                }
            }
            p { class: "text-sm font-medium text-slate-400 mb-1", "{title}" }
            if let Some(sub) = subtitle {
                p { class: "text-xs text-slate-600 mb-4", "{sub}" }
            }
            {children}
        }
    }
}
