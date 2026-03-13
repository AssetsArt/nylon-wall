use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::*;

// ─── Color Palette ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Emerald,
    Red,
    Amber,
    Slate,
    Blue,
    Violet,
    Cyan,
    Teal,
    Purple,
}

impl Color {
    pub fn badge_class(self) -> &'static str {
        match self {
            Color::Emerald => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
            Color::Red     => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/10 text-red-400 border border-red-500/20",
            Color::Amber   => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20",
            Color::Slate   => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-500/10 text-slate-400 border border-slate-500/20",
            Color::Blue    => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20",
            Color::Violet  => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20",
            Color::Cyan    => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-cyan-500/10 text-cyan-400 border border-cyan-500/20",
            Color::Teal    => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-teal-500/10 text-teal-400 border border-teal-500/20",
            Color::Purple  => "px-2 py-0.5 rounded-full text-[11px] font-medium bg-purple-500/10 text-purple-400 border border-purple-500/20",
        }
    }

    pub fn stat_card_class(self) -> &'static str {
        match self {
            Color::Emerald => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-emerald-500/30 transition-colors",
            Color::Red     => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-red-500/30 transition-colors",
            Color::Amber   => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-amber-500/30 transition-colors",
            Color::Slate   => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-slate-500/30 transition-colors",
            Color::Blue    => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-blue-500/30 transition-colors",
            Color::Violet  => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-violet-500/30 transition-colors",
            Color::Cyan    => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-cyan-500/30 transition-colors",
            Color::Teal    => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-teal-500/30 transition-colors",
            Color::Purple  => "rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-purple-500/30 transition-colors",
        }
    }

    pub fn icon_bg_class(self) -> &'static str {
        match self {
            Color::Emerald => "w-9 h-9 rounded-lg bg-emerald-500/10 flex items-center justify-center",
            Color::Red     => "w-9 h-9 rounded-lg bg-red-500/10 flex items-center justify-center",
            Color::Amber   => "w-9 h-9 rounded-lg bg-amber-500/10 flex items-center justify-center",
            Color::Slate   => "w-9 h-9 rounded-lg bg-slate-500/10 flex items-center justify-center",
            Color::Blue    => "w-9 h-9 rounded-lg bg-blue-500/10 flex items-center justify-center",
            Color::Violet  => "w-9 h-9 rounded-lg bg-violet-500/10 flex items-center justify-center",
            Color::Cyan    => "w-9 h-9 rounded-lg bg-cyan-500/10 flex items-center justify-center",
            Color::Teal    => "w-9 h-9 rounded-lg bg-teal-500/10 flex items-center justify-center",
            Color::Purple  => "w-9 h-9 rounded-lg bg-purple-500/10 flex items-center justify-center",
        }
    }

    pub fn btn_class(self) -> &'static str {
        match self {
            Color::Blue    => "px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors",
            Color::Red     => "px-3 py-1.5 rounded-lg text-xs font-medium bg-red-500/10 text-red-400 border border-red-500/20 hover:bg-red-500/20 transition-colors",
            Color::Emerald => "px-3 py-1.5 rounded-lg text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20 transition-colors",
            Color::Violet  => "px-3 py-1.5 rounded-lg text-xs font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20 hover:bg-violet-500/20 transition-colors",
            Color::Purple  => "px-3 py-1.5 rounded-lg text-xs font-medium bg-purple-500/10 text-purple-400 border border-purple-500/20 hover:bg-purple-500/20 transition-colors",
            Color::Amber   => "px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20 hover:bg-amber-500/20 transition-colors",
            Color::Slate   => "px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800/50 text-slate-400 border border-slate-700/40 hover:bg-slate-700/50 transition-colors",
            Color::Cyan    => "px-3 py-1.5 rounded-lg text-xs font-medium bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 hover:bg-cyan-500/20 transition-colors",
            Color::Teal    => "px-3 py-1.5 rounded-lg text-xs font-medium bg-teal-500/10 text-teal-400 border border-teal-500/20 hover:bg-teal-500/20 transition-colors",
        }
    }

    pub fn submit_btn_class(self) -> &'static str {
        match self {
            Color::Blue    => "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
            Color::Red     => "px-4 py-2 rounded-lg text-sm font-medium bg-red-500/10 text-red-400 border border-red-500/20 hover:bg-red-500/20 transition-colors disabled:opacity-50",
            Color::Violet  => "px-4 py-2 rounded-lg text-sm font-medium bg-violet-500/10 text-violet-400 border border-violet-500/20 hover:bg-violet-500/20 transition-colors disabled:opacity-50",
            Color::Purple  => "px-4 py-2 rounded-lg text-sm font-medium bg-purple-500/10 text-purple-400 border border-purple-500/20 hover:bg-purple-500/20 transition-colors disabled:opacity-50",
            _              => "px-4 py-2 rounded-lg text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors disabled:opacity-50",
        }
    }

}

// ─── Badge ───────────────────────────────────────────────────────────────────

#[component]
pub fn Badge(color: Color, label: String) -> Element {
    rsx! {
        span { class: color.badge_class(), "{label}" }
    }
}

// ─── Buttons ─────────────────────────────────────────────────────────────────

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

// ─── Stat Card ───────────────────────────────────────────────────────────────

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

// ─── Page / Section Headers ──────────────────────────────────────────────────

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

// ─── Error Alert ─────────────────────────────────────────────────────────────

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

// ─── Form Components ─────────────────────────────────────────────────────────

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
pub fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "text-xs font-medium text-slate-400 mb-1.5 block", "{label}" }
            {children}
        }
    }
}

/// Standard text input styling constant
pub const INPUT_CLASS: &str = "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";
pub const SELECT_CLASS: &str = "w-full bg-slate-900 border border-slate-700/60 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none focus:border-blue-500/60 transition-colors";

// ─── Data Table ──────────────────────────────────────────────────────────────

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

// ─── Empty State (Standalone Card) ───────────────────────────────────────────

#[component]
pub fn EmptyState(
    icon: Element,
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] children: Element,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-dashed border-slate-800/60 p-12 text-center",
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

// ─── Pagination ──────────────────────────────────────────────────────────────

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
