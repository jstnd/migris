use gpui::{
    App, AppContext, Context, Entity, IntoElement, RenderOnce, SharedString, Styled, Window,
};
use gpui_component::{
    ActiveTheme,
    input::{Input, InputState, TabSize},
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

        Input::new(&state.input_state)
            .p_0()
            .h_full()
            .appearance(false)
            .font_family(cx.theme().mono_font_family.clone())
    }
}
