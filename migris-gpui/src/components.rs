use gpui_kit::{
    App, BorrowAppContext, InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled,
    base::{h_flex, v_flex},
    component::{ActiveTheme, WindowExt, dialog::AlertDialog},
    div,
};

use crate::{
    components::icon::{Icon, IconName},
    state::AppState,
};

pub mod connection_dialog;
pub mod connection_panel;
pub mod editor;
pub mod icon;
pub mod settings;
pub mod tab_panel;
pub mod table;

/// Initializes configuration for components.
pub fn init(cx: &mut App) {
    connection_panel::init(cx);
    editor::init(cx);
    table::init(cx);
}

pub fn entry_screen(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_1()
        .size_full()
        .items_center()
        .justify_center()
        .child(div().text_3xl().child(Icon::primary(cx, IconName::DatabaseX)))
        .child(
            h_flex()
                .child("No connection open; ")
                .child(
                    div()
                        .id("open-connection")
                        .cursor_pointer()
                        .hover(|style| style.underline())
                        .text_color(cx.theme().link)
                        .child("add or open a connection")
                        .on_click(|_, window, cx| {
                            // Load the connection dialog first before displaying.
                            cx.update_global(|app_state: &mut AppState, cx| {
                                app_state.load_connection_dialog(cx);
                            });

                            window.open_dialog(cx, |dialog, window, cx| {
                                connection_dialog::connection_dialog(dialog, window, cx)
                            });
                        }),
                )
                .child("."),
        )
}

pub fn error_dialog(dialog: AlertDialog, cx: &mut App, error: &str) -> AlertDialog {
    dialog
        .title(h_flex().gap_2().child(Icon::red(cx, IconName::CircleX2)).child("Error"))
        .description(SharedString::new(error))
}

pub fn labeled(label: impl Into<SharedString>, element: impl IntoElement) -> impl IntoElement {
    v_flex()
        .gap_0p5()
        .w_full()
        .text_sm()
        .child(h_flex().pl_1().child(label.into()))
        .child(h_flex().child(element))
}

pub fn text_ellipsis(element: impl IntoElement) -> impl IntoElement {
    div().truncate().child(element)
}
