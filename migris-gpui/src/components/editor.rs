use gpui::{
    App, AppContext, Context, Entity, IntoElement, RenderOnce, SharedString, Styled, Window,
};
use gpui_component::{
    ActiveTheme,
    input::{self, Input, InputState, TabSize},
};

use crate::{
    components::icon::{IconName, icon},
    event::AppAction,
};

/// The state for use with an [`Editor`].
pub struct EditorState {
    /// The state for the editor input.
    input_state: Entity<InputState>,
}

impl EditorState {
    /// Creates a new [`EditorState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("sql")
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
        });

        Self { input_state }
    }

    /// Returns whether the editor is empty, excluding whitespace.
    pub fn is_empty(&self, cx: &App) -> bool {
        self.input_state.read(cx).value().trim().is_empty()
    }

    /// Returns the selected content within the editor.
    pub fn selected_value(&self, cx: &App) -> SharedString {
        self.input_state.read(cx).selected_value()
    }

    /// Returns the content within the editor.
    pub fn value(&self, cx: &App) -> SharedString {
        self.input_state.read(cx).value()
    }
}

/// An editor component for handling SQL editing.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
}

impl Editor {
    /// Creates a new [`Editor`] with the given state.
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_empty = state.is_empty(cx);
        let is_selected_empty = state.selected_value(cx).is_empty();

        Input::new(&state.input_state)
            .p_0()
            .h_full()
            .appearance(false)
            .font_family(cx.theme().mono_font_family.clone())
            .context_menu(move |menu, _, cx| {
                menu.menu_with_icon_and_disabled(
                    "Run",
                    icon(cx, IconName::Play, is_empty),
                    Box::new(AppAction::RunSql),
                    is_empty,
                )
                .menu_with_icon_and_disabled(
                    "Run Selection",
                    icon(cx, IconName::MousePointer2, is_selected_empty),
                    Box::new(AppAction::RunSqlSelection),
                    is_selected_empty,
                )
                .separator()
                .menu("Cut", Box::new(input::Cut))
                .menu("Copy", Box::new(input::Copy))
                .menu("Paste", Box::new(input::Paste))
                .menu("Select All", Box::new(input::SelectAll))
            })
    }
}
