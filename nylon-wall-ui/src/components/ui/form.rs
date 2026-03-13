use dioxus::prelude::*;

pub const INPUT_CLASS: &str = "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";
pub const SELECT_CLASS: &str = "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";

#[component]
pub fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "{label}" }
            {children}
        }
    }
}
