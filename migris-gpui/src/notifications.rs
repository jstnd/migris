use gpui_kit::{App, Window, component::WindowExt};

use crate::components;

/// Displays a dialog containing the given error.
pub fn show_error(window: &mut Window, cx: &mut App, error: String) {
    window.open_alert_dialog(cx, move |dialog, _, cx| {
        components::error_dialog(dialog, cx, &error)
    });
}
