use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, SharedString,
    Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon,
    button::Button,
    h_flex,
    resizable::{resizable_panel, v_resizable},
    v_flex,
};
use migris::QueryResult;

use crate::{
    components::{
        editor::{Editor, EditorState},
        icon::IconName,
        table::{QueryTable, QueryTableState},
    },
    event::TabEvent,
};

struct QueryTabState {
    /// The state for the editor.
    editor_state: Entity<EditorState>,

    /// The state for the table showing query results.
    table: Option<Entity<QueryTableState>>,
}

impl EventEmitter<TabEvent> for QueryTabState {}

impl QueryTabState {
    /// Creates a new [`QueryTabState`].
    fn new(window: &mut Window, cx: &mut App) -> Self {
        let editor_state = cx.new(|cx| EditorState::new(window, cx));

        Self {
            editor_state,
            table: None,
        }
    }

    /// Loads the given query result into the tab.
    fn load_result(&mut self, window: &mut Window, cx: &mut Context<Self>, result: QueryResult) {
        let table = cx.new(|cx| QueryTableState::new(window, cx, result.data));
        self.table = Some(table);
    }

    /// Triggers an event to run the query in the editor.
    fn run_query(&self, cx: &mut Context<Self>) {
        let query = self.editor_state.read(cx).value(cx);
        cx.emit(TabEvent::RunQuery(query));
    }
}

pub struct QueryTab {
    /// The state for the query tab.
    state: Entity<QueryTabState>,

    /// The query tab label.
    label: SharedString,

    /// The query tab number.
    number: usize,

    /// The subscription for the tab.
    ///
    /// This will mainly be used for emitting events from the tab upwards.
    _subscription: Subscription,
}

impl EventEmitter<TabEvent> for QueryTab {}

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
            number,
            _subscription,
        }
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_editor_empty = state.editor_state.read(cx).is_empty(cx);

        v_resizable(format!("query-tab-{}", self.number))
            .child(
                resizable_panel().child(
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
                                    .on_click(window.listener_for(
                                        &self.state,
                                        |state, _, _, cx| {
                                            state.run_query(cx);
                                        },
                                    )),
                            ),
                        )
                        .child(Editor::new(&state.editor_state)),
                ),
            )
            .child(resizable_panel().when(state.table.is_some(), |this| {
                this.child(QueryTable::new(state.table.as_ref().unwrap()))
            }))
    }

    /// Returns the label for the tab.
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }

    /// Loads the given query result into the tab.
    pub fn load_result(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        result: QueryResult,
    ) {
        self.state.update(cx, |this, cx| {
            this.load_result(window, cx, result);
        });
    }
}
