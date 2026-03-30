use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, SharedString,
    Styled, Subscription, Window,
};
use gpui_component::{ActiveTheme, Disableable, Icon, button::Button, h_flex, v_flex};

use crate::{
    app::ApplicationEvent,
    components::{
        editor::{Editor, EditorState},
        icon::IconName,
    },
};

struct QueryTabState {
    /// The state for the editor.
    editor_state: Entity<EditorState>,
}

impl EventEmitter<ApplicationEvent> for QueryTabState {}

impl QueryTabState {
    /// Creates a new [`QueryTabState`].
    fn new(window: &mut Window, cx: &mut App) -> Self {
        let editor_state = cx.new(|cx| EditorState::new(window, cx));
        Self { editor_state }
    }

    fn run_query(&self, cx: &mut Context<Self>) {
        let query = self.editor_state.read(cx).value(cx);
        cx.emit(ApplicationEvent::RunQuery(query));
    }
}

pub struct QueryTab {
    /// The state for the query tab.
    state: Entity<QueryTabState>,

    /// The query tab label.
    label: SharedString,

    /// The subscription for the tab.
    ///
    /// This will mainly be used for emitting events from the tab upwards.
    _subscription: Subscription,
}

impl EventEmitter<ApplicationEvent> for QueryTab {}

impl QueryTab {
    /// Creates a new [`QueryTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, number: usize) -> Self {
        let state = cx.new(|cx| QueryTabState::new(window, cx));
        let _subscription = cx.subscribe(&state, |_, _, event, cx| {
            // Emit the event upwards.
            cx.emit(event.clone());
        });

        Self {
            state,
            label: SharedString::from(format!("Query #{}", number)),
            _subscription,
        }
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_editor_empty = state.editor_state.read(cx).is_empty(cx);

        v_flex()
            .pt_1()
            .gap_1()
            .size_full()
            .child(
                h_flex().pl_1().child(
                    Button::new("button-run")
                        .icon(Icon::from(IconName::Play).text_color({
                            let opacity = if is_editor_empty { 0.25 } else { 1.0 };
                            cx.theme().button_primary.opacity(opacity)
                        }))
                        .label("Run")
                        .compact()
                        .disabled(is_editor_empty)
                        .on_click(window.listener_for(&self.state, |state, _, _, cx| {
                            state.run_query(cx);
                        })),
                ),
            )
            .child(Editor::new(&state.editor_state))
    }

    /// Returns the label for the tab.
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }
}
