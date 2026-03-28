use gpui::{App, AppContext, Context, Entity, IntoElement, RenderOnce, Styled, Window};
use gpui_component::input::{Input, InputState, TabSize};

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
            .font_family("Consolas")
    }
}
