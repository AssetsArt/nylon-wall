use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

use super::Color;

#[component]
pub fn Btn(
    color: Color,
    label: String,
    onclick: EventHandler<MouseEvent>,
    #[props(default = false)] disabled: bool,
    #[props(default)] icon: Option<Element>,
) -> Element {
    rsx! {
        button {
            class: "{color.btn_class()} flex items-center",
            disabled: disabled,
            onclick: move |e| onclick.call(e),
            if let Some(ic) = icon {
                {ic}
                span { class: "ml-1.5", "{label}" }
            } else {
                "{label}"
            }
        }
    }
}

#[component]
pub fn SubmitBtn(
    color: Color,
    label: String,
    onclick: EventHandler<MouseEvent>,
    #[props(default = false)] disabled: bool,
) -> Element {
    rsx! {
        button {
            class: color.submit_btn_class(),
            disabled: disabled,
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}

#[component]
pub fn IconBtn(
    #[props(default = "text-slate-400 hover:text-slate-200 p-1.5 rounded-lg hover:bg-slate-800/50 transition-colors")]
    class: &'static str,
    title: Option<String>,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: class,
            title: title.unwrap_or_default(),
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

#[component]
pub fn DeleteBtn(onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "flex items-center gap-1 px-2.5 py-1 rounded-lg text-[11px] font-medium text-red-400 hover:bg-red-500/10 transition-colors",
            onclick: move |e| onclick.call(e),
            Icon { width: 12, height: 12, icon: LdTrash2 }
            "Delete"
        }
    }
}

#[component]
pub fn RefreshBtn(onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "flex items-center px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-400 border border-slate-700/40 hover:bg-slate-700/50 transition-colors",
            onclick: move |e| onclick.call(e),
            Icon { width: 12, height: 12, icon: LdRefreshCw }
            span { class: "ml-1.5", "Refresh" }
        }
    }
}
