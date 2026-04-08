use gpui::{App, SharedString};

use crate::{assets::Themes, config::AppSettings};

/// Applies the given theme to the application.
/// - Saves the theme in the application's settings.
/// - Applies the theme's config to the application.
pub fn apply_theme(cx: &mut App, theme: SharedString) {
    AppSettings::global_mut(cx).theme = theme.clone();
    Themes::apply(cx, theme);
}
