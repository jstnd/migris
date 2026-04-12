use std::fmt::Display;

use futures_lite::StreamExt;
use gpui::{App, Global, SharedString};
use gpui_component::{Theme, ThemeMode};
use mundy::{ColorScheme, Interest, Preferences};

#[derive(Default)]
pub struct AppSettings {
    /// The app theme mode.
    pub theme_mode: AppThemeMode,

    /// The theme to use when dark mode is enabled.
    pub theme_dark: SharedString,

    /// The theme to use when light mode is enabled.
    pub theme_light: SharedString,
}

impl Global for AppSettings {}

impl AppSettings {
    /// Returns a reference to the global [`AppSettings`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns a mutable reference to the global [`AppSettings`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Returns the matching [`ThemeMode`] for the saved [`AppThemeMode`].
    pub fn theme_mode(&self, cx: &App) -> ThemeMode {
        match self.theme_mode {
            AppThemeMode::Dark => ThemeMode::Dark,
            AppThemeMode::Light => ThemeMode::Light,
            AppThemeMode::System => AppState::global(cx).system_theme_mode,
        }
    }

    /// Returns the saved theme for the current [`ThemeMode`].
    pub fn theme(&self, cx: &App) -> SharedString {
        match self.theme_mode(cx) {
            ThemeMode::Dark => self.theme_dark.clone(),
            ThemeMode::Light => self.theme_light.clone(),
        }
    }
}

#[derive(Default)]
pub struct AppState {
    ///
    system_theme_mode: ThemeMode,
}

impl Global for AppState {}

impl AppState {
    /// Initializes functionality needed for the global [`AppState`].
    pub fn init(cx: &mut App) {
        cx.spawn(async |cx| {
            // Listen to changes in the system theme; this is needed for
            // when the user has the system app theme mode selected.
            let mut stream = Preferences::stream(Interest::ColorScheme);

            while let Some(preferences) = stream.next().await {
                cx.update_global(|state: &mut AppState, cx| {
                    state.system_theme_mode = match preferences.color_scheme {
                        ColorScheme::Dark => ThemeMode::Dark,
                        ColorScheme::Light | ColorScheme::NoPreference => ThemeMode::Light,
                    };

                    if AppSettings::global(cx).theme_mode.is_system() {
                        Theme::change(state.system_theme_mode, None, cx);
                    }
                });
            }
        })
        .detach();
    }

    /// Returns a reference to the global [`AppState`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns a mutable reference to the global [`AppState`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum AppThemeMode {
    Dark,
    Light,
    #[default]
    System,
}

impl AppThemeMode {
    /// Returns a list of all app theme modes.
    ///
    /// Intended for use with dropdown components.
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

    /// Returns whether this is the system app theme mode.
    pub fn is_system(&self) -> bool {
        *self == AppThemeMode::System
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
