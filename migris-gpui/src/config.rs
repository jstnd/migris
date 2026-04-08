use std::fmt::Display;

use gpui::{App, Global, SharedString};
use gpui_component::ThemeMode;

#[derive(Default)]
pub struct AppSettings {
    pub theme_mode: AppThemeMode,
    pub theme: SharedString,
}

impl Global for AppSettings {}

impl AppSettings {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn theme_mode(&self, cx: &App) -> ThemeMode {
        self.theme_mode.theme_mode(cx)
    }
}

#[derive(Default)]
pub struct AppState {
    system_theme_mode: ThemeMode,
}

impl Global for AppState {}

impl AppState {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

#[derive(Clone, Copy, Default)]
pub enum AppThemeMode {
    Dark,
    Light,
    #[default]
    System,
}

impl AppThemeMode {
    pub fn options() -> Vec<(SharedString, SharedString)> {
        let options = [Self::Dark, Self::Light, Self::System];

        options
            .iter()
            .map(|option| {
                let option = option.to_string();
                (SharedString::from(&option), SharedString::from(&option))
            })
            .collect()
    }

    pub fn theme_mode(&self, cx: &App) -> ThemeMode {
        match self {
            AppThemeMode::Dark => ThemeMode::Dark,
            AppThemeMode::Light => ThemeMode::Light,
            AppThemeMode::System => AppState::global(cx).system_theme_mode,
        }
    }
}

impl Display for AppThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::System => "System",
        };

        write!(f, "{}", display)
    }
}

impl From<SharedString> for AppThemeMode {
    fn from(value: SharedString) -> Self {
        match value.as_str() {
            "Dark" => Self::Dark,
            "Light" => Self::Light,
            "System" => Self::System,
            _ => Self::System,
        }
    }
}
