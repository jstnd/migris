use gpui::{App, IntoElement, ParentElement, SharedString, Styled, div};
use gpui_component::{dialog::AlertDialog, h_flex, v_flex};

use crate::components::icon::{Icon, IconName};

pub mod connections;
pub mod editor;
pub mod icon;
pub mod panels;
pub mod settings;
pub mod table;

/// Initializes configuration for components.
pub fn init(cx: &mut App) {
    panels::init(cx);
}

pub fn error_dialog(dialog: AlertDialog, cx: &mut App, error: String) -> AlertDialog {
    dialog
        .title(
            h_flex()
                .gap_2()
                .child(Icon::danger(cx, IconName::CircleX2))
                .child("Error"),
        )
        .description(error)
        .overlay_closable(true)
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
    div()
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .child(element)
}
