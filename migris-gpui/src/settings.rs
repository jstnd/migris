use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::anyhow;
use directories::BaseDirs;
use gpui::{App, Global, SharedString};
use gpui_component::{ActiveTheme, Theme, ThemeMode};
use serde::{Deserialize, Serialize};

use crate::{assets::Themes, shared, size::Size, state::AppState};

pub struct SettingsManager {
    settings: Settings,
}

impl Global for SettingsManager {}

impl SettingsManager {
    /// Loads from the settings file.
    pub fn load(cx: &mut App) -> Self {
        Self {
            settings: Self::load_settings(cx),
        }
    }

    /// Reloads from the settings file.
    pub fn reload(cx: &mut App) {
        Self::global_mut(cx).settings = Self::load_settings(cx);
    }

    fn load_settings(cx: &mut App) -> Settings {
        let mut settings = Self::try_load().unwrap_or_else(|_| Settings::default());
        settings.verify_themes(cx);
        settings.apply(cx);
        settings
    }

    /// Saves to the settings file.
    pub fn save(cx: &App) {
        // TODO: log errors with saving
        _ = Self::global(cx).try_save();
    }

    /// Retrieves the path for the settings file.
    fn settings_path() -> Result<PathBuf, anyhow::Error> {
        let Some(dirs) = BaseDirs::new() else {
            return Err(anyhow!("Failed to retrieve directories"));
        };

        Ok(dirs
            .config_dir()
            .join(shared::APPLICATION_NAME)
            .join("settings.json"))
    }

    fn try_load() -> Result<Settings, anyhow::Error> {
        let path = Self::settings_path()?;
        let reader = BufReader::new(File::open(path)?);
        Ok(serde_json::from_reader(reader)?)
    }

    fn try_save(&self) -> Result<(), anyhow::Error> {
        let path = Self::settings_path()?;
        let writer = BufWriter::new(File::create(path)?);
        serde_json::to_writer_pretty(writer, &self.settings)?;
        Ok(())
    }

    /// Returns a reference to the global [`AppSettings`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns a mutable reference to the global [`AppSettings`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Returns the saved [`AppThemeMode`].
    pub fn app_theme_mode(cx: &App) -> AppThemeMode {
        Self::global(cx).settings.appearance.theme_mode
    }

    /// Returns the saved [`Size`] for the editor.
    pub fn editor_size(cx: &App) -> Size {
        Self::global(cx).settings.appearance.editor_size
    }

    /// Sets the saved [`AppThemeMode`].
    pub fn set_app_theme_mode(cx: &mut App, mode: AppThemeMode) {
        Self::global_mut(cx).settings.appearance.theme_mode = mode;
    }

    /// Sets the saved [`Size`] for the editor.
    pub fn set_editor_size(cx: &mut App, size: Size) {
        Self::global_mut(cx).settings.appearance.editor_size = size;
    }

    /// Sets the saved theme for the current [`ThemeMode`].
    pub fn set_theme(cx: &mut App, theme: SharedString) {
        let mode = cx.theme().mode;
        let manager = Self::global_mut(cx);

        match mode {
            ThemeMode::Dark => manager.settings.appearance.theme_dark = theme,
            ThemeMode::Light => manager.settings.appearance.theme_light = theme,
        }
    }

    /// Returns the saved theme for the current [`ThemeMode`].
    pub fn theme(cx: &App) -> SharedString {
        Self::global(cx).settings.theme(cx)
    }

    /// Returns the matching [`ThemeMode`] for the saved [`AppThemeMode`].
    pub fn theme_mode(cx: &App) -> ThemeMode {
        Self::global(cx).settings.theme_mode(cx)
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Settings {
    appearance: AppearanceSettings,
}

impl Settings {
    /// Applies the saved values to the application.
    fn apply(&self, cx: &mut App) {
        Theme::change(self.theme_mode(cx), None, cx);
        Themes::apply(cx, self.theme(cx));
    }

    /// Returns the saved theme for the current [`ThemeMode`].
    fn theme(&self, cx: &App) -> SharedString {
        match self.theme_mode(cx) {
            ThemeMode::Dark => self.appearance.theme_dark.clone(),
            ThemeMode::Light => self.appearance.theme_light.clone(),
        }
    }

    /// Returns the matching [`ThemeMode`] for the saved [`AppThemeMode`].
    fn theme_mode(&self, cx: &App) -> ThemeMode {
        match self.appearance.theme_mode {
            AppThemeMode::Dark => ThemeMode::Dark,
            AppThemeMode::Light => ThemeMode::Light,
            AppThemeMode::System => AppState::global(cx).system_theme_mode,
        }
    }

    /// Verifies that the saved themes are valid.
    fn verify_themes(&mut self, cx: &App) {
        if !Themes::contains(cx, &self.appearance.theme_dark) {
            self.appearance.theme_dark = Themes::default(ThemeMode::Dark);
        }

        if !Themes::contains(cx, &self.appearance.theme_light) {
            self.appearance.theme_light = Themes::default(ThemeMode::Light);
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct AppearanceSettings {
    /// The size of the editor.
    editor_size: Size,

    /// The theme to use when dark mode is enabled.
    theme_dark: SharedString,

    /// The theme to use when light mode is enabled.
    theme_light: SharedString,

    /// The app theme mode.
    theme_mode: AppThemeMode,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
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
