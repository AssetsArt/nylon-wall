use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

const PAGE_BTN: &str = "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors disabled:opacity-30 bg-slate-800/50 text-slate-400 border-slate-700/40 hover:bg-slate-700/50";
const PAGE_BTN_ACTIVE: &str = "px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors bg-blue-500/20 text-blue-400 border-blue-500/30";

#[component]
pub fn Pagination(
    current: usize,
    total_pages: usize,
    on_change: EventHandler<usize>,
) -> Element {
    if total_pages <= 1 {
        return rsx! {};
    }

    let end = (current + 3).min(total_pages);
    let start = end.saturating_sub(5);

    rsx! {
        div { class: "flex items-center justify-between mt-4",
            div { class: "flex items-center gap-1",
                button {
                    class: PAGE_BTN,
                    disabled: current == 0,
                    onclick: move |_| on_change.call(0),
                    Icon { width: 12, height: 12, icon: LdChevronsLeft }
                }
                button {
                    class: PAGE_BTN,
                    disabled: current == 0,
                    onclick: move |_| on_change.call(current.saturating_sub(1)),
                    Icon { width: 12, height: 12, icon: LdChevronLeft }
                }

                for p in start..end {
                    button {
                        key: "{p}",
                        class: if p == current { PAGE_BTN_ACTIVE } else { PAGE_BTN },
                        onclick: move |_| on_change.call(p),
                        "{p + 1}"
                    }
                }

                button {
                    class: PAGE_BTN,
                    disabled: current + 1 >= total_pages,
                    onclick: move |_| on_change.call(current + 1),
                    Icon { width: 12, height: 12, icon: LdChevronRight }
                }
                button {
                    class: PAGE_BTN,
                    disabled: current + 1 >= total_pages,
                    onclick: move |_| on_change.call(total_pages - 1),
                    Icon { width: 12, height: 12, icon: LdChevronsRight }
                }
            }

            span { class: "text-xs text-slate-600",
                "Page {current + 1} of {total_pages}"
            }
        }
    }
}
