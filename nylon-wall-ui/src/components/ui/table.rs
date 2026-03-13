use dioxus::prelude::*;

pub const TH_CLASS: &str = "px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-slate-500";
pub const TD_CLASS: &str = "px-5 py-3 text-sm";
pub const TR_CLASS: &str = "border-t border-slate-800/40 hover:bg-slate-800/30 transition-colors";

#[component]
pub fn DataTable(children: Element) -> Element {
    rsx! {
        div { class: "rounded-xl border border-slate-800/60 overflow-hidden",
            table { class: "w-full text-left",
                {children}
            }
        }
    }
}

#[component]
pub fn TableEmpty(
    #[props(default = 6)] colspan: u32,
    message: String,
) -> Element {
    rsx! {
        tr {
            td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "{colspan}",
                "{message}"
            }
        }
    }
}

#[component]
pub fn TableError(
    #[props(default = 6)] colspan: u32,
    message: String,
) -> Element {
    rsx! {
        tr {
            td { class: "px-5 py-16 text-center text-sm text-red-400", colspan: "{colspan}",
                "{message}"
            }
        }
    }
}

#[component]
pub fn TableLoading(
    #[props(default = 6)] colspan: u32,
) -> Element {
    rsx! {
        tr {
            td { class: "px-5 py-16 text-center text-sm text-slate-600", colspan: "{colspan}",
                "Loading..."
            }
        }
    }
}
