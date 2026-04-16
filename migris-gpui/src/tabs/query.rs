use gpui::{
    App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Disableable,
    button::{Button, DropdownButton},
    h_flex,
    resizable::{resizable_panel, v_resizable},
    tab::{Tab, TabBar},
    v_flex,
};
use migris::data::QueryResult;

use crate::{
    components::{
        editor::{Editor, EditorState},
        icon::{IconName, icon},
        table::{QueryTable, QueryTableState},
    },
    event::{AppAction, AppEvent, AppEventKind, RunSql},
};

struct QueryTabState {
    /// The state for the editor.
    editor_state: Entity<EditorState>,

    /// The states for the tables showing query results.
    tables: Vec<Entity<QueryTableState>>,

    /// The index of the active table tab.
    active_table: usize,
}

impl EventEmitter<AppEvent> for QueryTabState {}

impl QueryTabState {
    /// Creates a new [`QueryTabState`].
    fn new(window: &mut Window, cx: &mut App) -> Self {
        let editor_state = cx.new(|cx| EditorState::new(window, cx));

        Self {
            editor_state,
            tables: Vec::new(),
            active_table: 0,
        }
    }

    /// Handles actions originating from the tab.
    fn handle_action(&mut self, action: &AppAction, cx: &mut Context<Self>) {
        match action {
            AppAction::RunSql => {
                self.clear_results();
                self.run_sql(cx, false);
            }
            AppAction::RunSqlSelection => {
                self.clear_results();
                self.run_sql(cx, true);
            }
        }
    }

    /// Returns a reference to the active table.
    fn active_table(&self) -> &Entity<QueryTableState> {
        &self.tables[self.active_table]
    }

    /// Clears the results from the tab.
    fn clear_results(&mut self) {
        self.tables.clear();
        self.active_table = 0;
    }

    /// Loads the given query result into the tab.
    fn load_result(&mut self, window: &mut Window, cx: &mut Context<Self>, result: QueryResult) {
        let table = cx.new(|cx| QueryTableState::new(window, cx, result));
        self.tables.push(table);
    }

    /// Triggers an event to run the SQL in the editor.
    fn run_sql(&self, cx: &mut Context<Self>, selected: bool) {
        let editor_state = self.editor_state.read(cx);
        let sql = if selected {
            editor_state.selected_value(cx)
        } else {
            editor_state.value(cx)
        };

        let event = AppEvent::new(AppEventKind::RunSql(RunSql {
            sql,
            show_progress: true,
            stream: false,
        }));

        cx.emit(event);
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

impl EventEmitter<AppEvent> for QueryTab {}

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
        self.state.update(cx, |state, cx| {
            state.load_result(window, cx, result);
        });
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_editor_empty = state.editor_state.read(cx).is_empty(cx);
        let is_editor_selected_empty = state.editor_state.read(cx).selected_value(cx).is_empty();

        v_resizable(format!("query-tab-{}", self.number))
            .child(
                resizable_panel().child(
                    v_flex()
                        .pt_1()
                        .gap_1()
                        .size_full()
                        .on_action(window.listener_for(&self.state, |state, action, _, cx| {
                            state.handle_action(action, cx);
                        }))
                        .child(
                            h_flex().pl_1().child(
                                DropdownButton::new("run-buttons")
                                    .compact()
                                    .disabled(is_editor_empty)
                                    .button(
                                        Button::new("run-button")
                                            .icon(icon(cx, IconName::Play, is_editor_empty))
                                            .label("Run")
                                            .on_click(window.listener_for(
                                                &self.state,
                                                |state, _, _, cx| {
                                                    state.handle_action(&AppAction::RunSql, cx);
                                                },
                                            )),
                                    )
                                    .dropdown_menu(move |menu, _, cx| {
                                        menu.menu_with_icon(
                                            "Run",
                                            icon(cx, IconName::Play, false),
                                            Box::new(AppAction::RunSql),
                                        )
                                        .menu_with_icon_and_disabled(
                                            "Run Selection",
                                            icon(
                                                cx,
                                                IconName::MousePointer2,
                                                is_editor_selected_empty,
                                            ),
                                            Box::new(AppAction::RunSqlSelection),
                                            is_editor_selected_empty,
                                        )
                                    }),
                            ),
                        )
                        .child(Editor::new(&state.editor_state)),
                ),
            )
            .child(resizable_panel().when(!state.tables.is_empty(), |this| {
                this.child(
                    v_flex()
                        .size_full()
                        .child(
                            h_flex()
                                .id("table-tab-bar")
                                .w_full()
                                .bg(cx.theme().tab_bar)
                                .overflow_x_scroll()
                                .child(
                                    TabBar::new("table-tabs")
                                        .selected_index(state.active_table)
                                        .on_click(window.listener_for(
                                            &self.state,
                                            |state, idx, _, _| {
                                                state.active_table = *idx;
                                            },
                                        ))
                                        .children(state.tables.iter().enumerate().map(
                                            |(idx, _)| {
                                                Tab::new().label(format!("Result #{}", idx + 1))
                                            },
                                        )),
                                ),
                        )
                        .child(QueryTable::new(state.active_table())),
                )
            }))
    }
}
