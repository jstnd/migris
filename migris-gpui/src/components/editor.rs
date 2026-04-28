use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, IntoElement, RenderOnce, SharedString, Styled, Window,
    prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme,
    input::{Input, InputState, TabSize},
    menu::PopupMenu,
};

/// The state used with an [`Editor`].
pub struct EditorState {
    /// The state for the editor input.
    input: Entity<InputState>,
}

impl EditorState {
    /// Creates a new [`EditorState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("sql")
                .context_menu(false)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
        });

        Self { input }
    }

    /// Returns whether the editor is empty, excluding whitespace.
    pub fn is_empty(&self, cx: &App) -> bool {
        self.input.read(cx).value().trim().is_empty()
    }

    /// Returns the selected content within the editor.
    pub fn selected_value(&self, cx: &App) -> SharedString {
        self.input.read(cx).selected_value()
    }

    /// Returns the content within the editor.
    pub fn value(&self, cx: &App) -> SharedString {
        self.input.read(cx).value()
    }
}

/// An editor component for handling SQL editing.
#[derive(IntoElement)]
pub struct Editor {
    /// The state for the editor.
    state: Entity<EditorState>,

    /// The optional context menu builder.
    context_menu_builder:
        Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static>>,
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
        f: impl Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        Input::new(&state.input)
            .p_0()
            .h_full()
            .appearance(false)
            .font_family(cx.theme().mono_font_family.clone())
            .when_some(self.context_menu_builder, |this, context_menu_builder| {
                this.context_menu(move |menu, window, cx| context_menu_builder(menu, window, cx))
            })
    }
}
