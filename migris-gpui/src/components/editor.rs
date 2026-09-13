use std::rc::Rc;

use gpui_kit::{
    Action, App, AppContext, Context, DispatchPhase, Entity, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Pixels, RenderOnce, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement, Styled, Window,
    base::input::TabSize,
    component::{
        input,
        menu::{ContextMenuExt, PopupMenu},
    },
    div, px,
};

use crate::settings::SettingsManager;

const EDITOR_ID: &str = "EDITOR";

/// Initializes configuration for the editor component.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl--", EditorAction::DecreaseSize, Some(EDITOR_ID)),
        KeyBinding::new("ctrl-=", EditorAction::IncreaseSize, Some(EDITOR_ID)),
    ]);
}

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum EditorAction {
    DecreaseSize,
    IncreaseSize,
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let context_menu_builder = self.context_menu_builder.clone();
        let state = self.state.read(cx);

        // Handle scrolling events within the editor for the purpose of zoom in/out.
        window.on_mouse_event({
            let state = self.state.clone();
            move |event: &ScrollWheelEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture
                    || !event.secondary()
                    || !state.read(cx).is_hovered
                {
                    return;
                }

                let delta_y = event.delta.pixel_delta(px(1.0)).y;
                let action = if delta_y < Pixels::ZERO {
                    EditorAction::DecreaseSize
                } else {
                    EditorAction::IncreaseSize
                };

                state.update(cx, |state, cx| {
                    state.handle_action(window, cx, &action);
                });
                cx.stop_propagation();
            }
        });

        div()
            .id(EDITOR_ID)
            .key_context(EDITOR_ID)
            .size_full()
            .child(
                input::Editor::new(&state.editor)
                    .p_0()
                    .h_full()
                    .appearance(false)
                    .text_size(SettingsManager::editor_size(cx).font_size(cx)),
            )
            .on_action(
                window.listener_for(&self.state, |state, action, window, cx| {
                    state.handle_action(window, cx, action);
                }),
            )
            .on_hover(
                window.listener_for(&self.state, |state, is_hovered, _, cx| {
                    state.is_hovered = *is_hovered;
                    cx.notify();
                }),
            )
            .context_menu(move |menu, window, cx| {
                if let Some(context_menu_builder) = context_menu_builder.clone() {
                    context_menu_builder(menu, window, cx)
                } else {
                    menu
                }
            })
    }
}

/// The state used with an [`Editor`].
pub struct EditorState {
    /// The state for the editor input.
    editor: Entity<input::EditorState>,

    /// Whether the editor is hovered over.
    is_hovered: bool,
}

impl EditorState {
    /// Creates a new [`EditorState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            input::EditorState::new(window, cx)
                .context_menu(false)
                .language("sql")
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
        });

        Self {
            editor,
            is_hovered: false,
        }
    }

    /// Handles actions originating from the editor.
    fn handle_action(&mut self, _: &mut Window, cx: &mut Context<Self>, action: &EditorAction) {
        match action {
            EditorAction::DecreaseSize | EditorAction::IncreaseSize => {
                let current_size = SettingsManager::editor_size(cx);
                let new_size = match action {
                    EditorAction::DecreaseSize => current_size.decrease(),
                    EditorAction::IncreaseSize => current_size.increase(),
                };

                if current_size != new_size {
                    SettingsManager::set_editor_size(cx, new_size);
                    SettingsManager::save(cx);
                    cx.notify();
                }
            }
        }
    }

    /// Focuses the editor input.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
        });
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
