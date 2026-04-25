use gpui::{IntoElement, ParentElement, SharedString, Styled, div};
use gpui_component::{h_flex, v_flex};

pub mod connections;
pub mod editor;
pub mod icon;
pub mod panels;
pub mod settings;
pub mod table;

///
pub fn labeled(label: impl Into<SharedString>, element: impl IntoElement) -> impl IntoElement {
    v_flex()
        .gap_0p5()
        .w_full()
        .text_sm()
        .child(h_flex().pl_1().child(label.into()))
        .child(element)
}
