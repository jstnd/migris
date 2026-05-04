use gpui::{App, Pixels, SharedString, px};
use gpui_component::{ActiveTheme, ThemeMode};

use crate::{assets::Themes, settings::AppSettings};

/// The application name.
pub const APPLICATION_NAME: &str = "Migris";

/// The application name in lowercase format.
pub const APPLICATION_NAME_LOWER: &str = "migris";

/// The width of primary dialogs (e.g. connections, settings).
pub const DIALOG_WIDTH: Pixels = px(800.0);

/// The height of primary dialogs (e.g. connections, settings).
pub const DIALOG_HEIGHT: Pixels = px(600.0);

/// The placeholder text for search input fields.
pub const SEARCH_PLACEHOLDER: &str = "Search...";

/// Applies the given theme to the application.
/// - Saves the theme in the application settings.
/// - Applies the theme's config to the application.
pub fn apply_theme(cx: &mut App, theme: SharedString) {
    match cx.theme().mode {
        ThemeMode::Dark => AppSettings::global_mut(cx).theme_dark = theme.clone(),
        ThemeMode::Light => AppSettings::global_mut(cx).theme_light = theme.clone(),
    }

    Themes::apply(cx, theme);
}
