use gpui::{App, AppContext, Element, Entity, ParentElement, SharedString, Styled, Window};
use gpui_component::v_flex;

use crate::{
    components::{
        editor::{Editor, EditorState},
        icon::IconName,
    },
    tabs::TabView,
};

pub struct QueryTab {
    /// The state for the editor.
    editor_state: Entity<EditorState>,

    /// The query tab label.
    label: SharedString,

    /// The query tab number.
    number: usize,
}

impl QueryTab {
    /// Creates a new [`QueryTab`].
    pub fn new(window: &mut Window, cx: &mut App, number: usize) -> Self {
        let editor_state = cx.new(|cx| EditorState::new(window, cx));

        Self {
            editor_state,
            label: SharedString::from(format!("Query #{}", number)),
            number,
        }
    }

    /// Returns the query tab's number.
    pub fn number(&self) -> usize {
        self.number
    }
}

impl TabView for QueryTab {
    fn icon(&self) -> IconName {
        IconName::Code
    }

    fn label(&self) -> SharedString {
        self.label.clone()
    }

    fn content(&self) -> gpui::AnyElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(Editor::new(&self.editor_state))
            .into_any()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
