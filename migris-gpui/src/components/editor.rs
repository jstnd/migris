use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, IntoElement, RenderOnce, SharedString, Styled, Window,
    prelude::FluentBuilder,
};
use gpui_component::{input, native_menu::NativeMenu};

/// The state used with an [`Editor`].
pub struct EditorState {
    /// The state for the editor input.
    editor: Entity<input::EditorState>,
}

impl EditorState {
    /// Creates a new [`EditorState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| input::EditorState::new(window, cx).language("sql"));
        Self { editor }
    }

    /// Returns whether the editor is empty, excluding whitespace.
    pub fn is_empty(&self, cx: &App) -> bool {
        self.editor.read(cx).value().trim().is_empty()
    }

    /// Returns the selected content within the editor.
    pub fn selected_value(&self, cx: &App) -> SharedString {
        self.editor.read(cx).selected_value()
    }

    /// Returns the content within the editor.
    pub fn value(&self, cx: &App) -> SharedString {
        self.editor.read(cx).value()
    }
}

/// An editor component for handling SQL editing.
#[derive(IntoElement)]
pub struct Editor {
    /// The state for the editor.
    state: Entity<EditorState>,

    /// The optional context menu builder.
    context_menu_builder:
        Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static>>,
}

impl Editor {
    /// Creates a new [`Editor`].
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            context_menu_builder: None,
        }
    }

    /// Sets the context menu for the editor.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        input::Editor::new(&state.editor)
            .p_0()
            .h_full()
            .appearance(false)
            .when_some(self.context_menu_builder, |this, context_menu_builder| {
                this.context_menu(move |menu, window, cx| context_menu_builder(menu, window, cx))
            })
    }
}
