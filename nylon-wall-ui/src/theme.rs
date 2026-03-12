use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            _ => Theme::Dark,
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
}

/// Initialize theme from localStorage and set data-theme attribute
pub fn use_theme_init() -> Signal<Theme> {
    let mut theme = use_signal(|| Theme::Dark);

    // On mount: read from localStorage
    use_effect(move || {
        spawn(async move {
            let result = document::eval(
                r#"
                let t = localStorage.getItem('nylon-wall-theme') || 'dark';
                document.documentElement.setAttribute('data-theme', t);
                return t;
                "#,
            );
            if let Ok(val) = result.await {
                if let Some(s) = val.as_str() {
                    theme.set(Theme::from_str(s));
                }
            }
        });
    });

    theme
}

/// Toggle theme and persist to localStorage
pub fn toggle_theme(mut theme: Signal<Theme>) {
    let new_theme = theme.read().toggle();
    theme.set(new_theme);
    let theme_str = new_theme.as_str();
    spawn(async move {
        let _ = document::eval(&format!(
            r#"
            document.documentElement.setAttribute('data-theme', '{theme_str}');
            localStorage.setItem('nylon-wall-theme', '{theme_str}');
            return true;
            "#,
        ))
        .await;
    });
}
