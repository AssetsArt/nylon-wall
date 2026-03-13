use dioxus::prelude::*;

#[component]
pub fn PageHeader(
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] children: Element,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between mb-6",
            div {
                h2 { class: "text-xl font-semibold text-white", "{title}" }
                if let Some(sub) = subtitle {
                    p { class: "text-sm text-slate-400 mt-1", "{sub}" }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn SectionHeader(
    icon: Element,
    title: String,
    #[props(default)] children: Element,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between mb-4",
            div { class: "flex items-center gap-2",
                div { class: "w-7 h-7 rounded-lg flex items-center justify-center", {icon} }
                h3 { class: "text-sm font-semibold text-white", "{title}" }
            }
            {children}
        }
    }
}
