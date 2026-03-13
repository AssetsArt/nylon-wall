use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

#[component]
pub fn ConfirmModal(
    title: String,
    message: String,
    confirm_label: Option<String>,
    danger: Option<bool>,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let label = confirm_label.unwrap_or_else(|| "Confirm".to_string());
    let is_danger = danger.unwrap_or(true);

    let (btn_cls, icon_bg, icon_color) = if is_danger {
        (
            "px-4 py-2 rounded-lg text-sm font-medium bg-red-500/20 text-red-400 border border-red-500/30 hover:bg-red-500/30 transition-colors",
            "bg-red-500/10 border border-red-500/20",
            "text-red-400",
        )
    } else {
        (
            "px-4 py-2 rounded-lg text-sm font-medium bg-amber-500/20 text-amber-400 border border-amber-500/30 hover:bg-amber-500/30 transition-colors",
            "bg-amber-500/10 border border-amber-500/20",
            "text-amber-400",
        )
    };

    rsx! {
        // Backdrop
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center",
            // Overlay
            div {
                class: "absolute inset-0 bg-black/60 backdrop-blur-sm",
                onclick: move |_| on_cancel.call(()),
            }
            // Modal card
            div { class: "relative z-10 w-full max-w-md mx-4 rounded-2xl border border-slate-700/60 bg-slate-900 shadow-2xl",
                div { class: "p-6",
                    // Icon + title
                    div { class: "flex items-start gap-4 mb-4",
                        div { class: "flex-shrink-0 w-10 h-10 rounded-xl {icon_bg} flex items-center justify-center",
                            if is_danger {
                                Icon { width: 20, height: 20, icon: LdTriangleAlert, class: "{icon_color}" }
                            } else {
                                Icon { width: 20, height: 20, icon: LdCircleAlert, class: "{icon_color}" }
                            }
                        }
                        div {
                            h3 { class: "text-base font-semibold text-white mb-1", "{title}" }
                            p { class: "text-sm text-slate-400 leading-relaxed", "{message}" }
                        }
                    }
                    // Buttons
                    div { class: "flex items-center justify-end gap-3 mt-6",
                        button {
                            class: "px-4 py-2 rounded-lg text-sm font-medium bg-slate-800/50 text-slate-400 border border-slate-700/40 hover:bg-slate-700/50 transition-colors",
                            onclick: move |_| on_cancel.call(()),
                            "Cancel"
                        }
                        button {
                            class: "{btn_cls}",
                            onclick: move |_| on_confirm.call(()),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
