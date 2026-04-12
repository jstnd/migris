use gpui::{App, SharedString};
use gpui_component::{ActiveTheme, ThemeMode};

use crate::{assets::Themes, config::AppSettings};

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
