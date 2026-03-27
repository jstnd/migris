use gpui::{Element, ParentElement, SharedString, Styled};
use gpui_component::v_flex;

use crate::{components::icon::IconName, tabs::TabView};

pub struct QueryTab;

impl TabView for QueryTab {
    fn icon(&self) -> IconName {
        IconName::Code
    }

    fn label(&self) -> SharedString {
        SharedString::from("Query #1")
    }

    fn content(&self) -> gpui::AnyElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child("QUERY TAB CONTENT")
            .into_any()
    }
}
