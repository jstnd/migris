use gpui::{
    App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable,
    button::{Button, DropdownButton},
    h_flex,
    resizable::{resizable_panel, v_resizable},
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{
    components::{
        editor::{Editor, EditorState},
        icon::{Icon, IconName},
        table::{QueryTable, QueryTableState},
    },
    events::{AppAction, Event, EventId, EventManager, RunSqlEvent},
};

struct QueryTabState {
    /// The state for the editor.
    editor_state: Entity<EditorState>,

    /// The states for the tables showing query results.
    tables: Vec<Entity<QueryTableState>>,

    /// The index of the active table tab.
    active_table: usize,
}

impl EventEmitter<EventId> for QueryTabState {}

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
    fn handle_action(&mut self, cx: &mut Context<Self>, action: &AppAction) {
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

    /// Triggers an event to run the SQL in the editor.
    fn run_sql(&self, cx: &mut Context<Self>, selected: bool) {
        let editor_state = self.editor_state.read(cx);
        let sql = if selected {
            editor_state.selected_value(cx)
        } else {
            editor_state.value(cx)
        };

        let this = cx.entity();
        let event = RunSqlEvent::new(sql)
            .show_progress()
            .on_result(move |result, window, cx| {
                this.update(cx, |this, cx| {
                    let table = cx.new(|cx| QueryTableState::new(window, cx, result));
                    this.tables.push(table);
                });
            });

        EventManager::emit(cx, Event::new(event));
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

impl EventEmitter<EventId> for QueryTab {}

impl QueryTab {
    /// Creates a new [`QueryTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, number: usize) -> Self {
        let state = cx.new(|cx| QueryTabState::new(window, cx));
        let _subscription = cx.subscribe(&state, |_, _, event, cx| {
            // Emit the event upwards.
            cx.emit(*event);
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

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_editor_empty = state.editor_state.read(cx).is_empty(cx);
        let is_editor_selected_empty = state.editor_state.read(cx).selected_value(cx).is_empty();

        v_resizable(format!("query-tab-{}", self.number))
            .child(
                resizable_panel().child(
                    v_flex()
                        .gap_1()
                        .pt_1()
                        .size_full()
                        .on_action(window.listener_for(&self.state, |state, action, _, cx| {
                            state.handle_action(cx, action);
                        }))
                        .child(
                            h_flex().pl_1().child(
                                DropdownButton::new("run-buttons")
                                    .disabled(is_editor_empty)
                                    .compact()
                                    .small()
                                    .button(
                                        Button::new("run-button")
                                            .icon(
                                                Icon::new(cx, IconName::Play)
                                                    .disabled(is_editor_empty)
                                                    .primary(cx),
                                            )
                                            .label("Run")
                                            .on_click(window.listener_for(
                                                &self.state,
                                                |state, _, _, cx| {
                                                    state.handle_action(cx, &AppAction::RunSql);
                                                },
                                            )),
                                    )
                                    .dropdown_menu(move |menu, _, cx| {
                                        menu.menu_with_icon(
                                            "Run",
                                            Icon::new(cx, IconName::Play).primary(cx),
                                            Box::new(AppAction::RunSql),
                                        )
                                        .menu_with_icon_and_disabled(
                                            "Run Selection",
                                            Icon::new(cx, IconName::MousePointer2)
                                                .disabled(is_editor_selected_empty)
                                                .primary(cx),
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
