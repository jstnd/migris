use std::rc::Rc;

use gpui_kit::{App, ParentElement, SharedString, Styled, Window, base::h_flex, component::WindowExt};

use crate::components::icon::{Icon, IconName};

/// Displays a confirmation dialog.
pub fn show_confirmation(
    window: &mut Window,
    cx: &mut App,
    title: &str,
    message: &str,
    on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    on_ok: impl Fn(&mut Window, &mut App) + 'static,
) {
    let title = SharedString::from(title);
    let message = SharedString::from(message);
    let on_cancel = Rc::new(on_cancel);
    let on_ok = Rc::new(on_ok);
    window.open_alert_dialog(cx, move |dialog, _, cx| {
        let on_cancel = on_cancel.clone();
        let on_ok = on_ok.clone();
        dialog
            .title(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::yellow(cx, IconName::CircleAlert))
                    .child(title.clone()),
            )
            .description(message.clone())
            .show_cancel(true)
            .cancel_text("No")
            .ok_text("Yes")
            .on_cancel(move |_, window, cx| {
                (on_cancel)(window, cx);
                true
            })
            .on_ok(move |_, window, cx| {
                (on_ok)(window, cx);
                true
            })
    });
}

/// Displays a dialog containing the given error.
pub fn show_error(window: &mut Window, cx: &mut App, error: String) {
    window.open_alert_dialog(cx, move |dialog, _, cx| {
        dialog
            .title(h_flex().gap_2().child(Icon::red(cx, IconName::CircleX2)).child("Error"))
            .description(SharedString::new(&error))
    });
}
