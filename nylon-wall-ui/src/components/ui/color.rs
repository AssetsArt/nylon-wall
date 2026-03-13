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
