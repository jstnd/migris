use gpui_kit::{
    Action, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    base::{Disableable, h_flex, resizable_panel, v_flex, v_resizable},
    component::{
        Sizable,
        button::{Button, DropdownButton},
        input,
        tab::{Tab, TabBar},
    },
    prelude::FluentBuilder,
};

use crate::{
    components::{
        editor::{Editor, EditorState},
        icon::{Icon, IconName},
        table::{QueryTable, QueryTableEvent, QueryTableState},
    },
    events::{Event, EventId, EventManager, RunSqlEvent},
    notifications,
};

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
pub enum QueryTabAction {
    FormatSql,
    RunSql,
    RunSqlSelection,
}

pub struct QueryTab {
    /// The state for the query tab.
    state: Entity<QueryTabState>,

    /// The query tab label.
    label: SharedString,

    /// The query tab number.
    number: usize,
}

impl QueryTab {
    /// Creates a new [`QueryTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, number: usize) -> Self {
        let state = cx.new(|cx| QueryTabState::new(window, cx));

        Self {
            state,
            label: SharedString::from(format!("Query #{}", number)),
            number,
        }
    }

    /// Performs any needed behavior for closing the tab.
    pub fn close(&self, cx: &App) {
        self.state.read(cx).cancel_event(cx);
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_editor_empty = state.editor.read(cx).is_empty(cx);
        let is_editor_selected_empty = state.editor.read(cx).selected_value(cx).is_empty();
        let has_active_event = state.active_event.is_some();

        let is_run_disabled = is_editor_empty || has_active_event;
        let is_cancel_disabled = !has_active_event;

        v_resizable(format!("query-tab-{}", self.number))
            .child(
                resizable_panel().child(
                    v_flex()
                        .size_full()
                        .child(
                            h_flex().p_1().child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        DropdownButton::new("run-buttons")
                                            .disabled(is_run_disabled)
                                            .small()
                                            .button(
                                                Button::new("btn-run-query")
                                                    .icon(
                                                        Icon::primary(cx, IconName::Play)
                                                            .disabled(is_run_disabled),
                                                    )
                                                    .label("Run")
                                                    .on_click(window.listener_for(
                                                        &self.state,
                                                        |state, _, window, cx| {
                                                            state.handle_action(
                                                                window,
                                                                cx,
                                                                &QueryTabAction::RunSql,
                                                            );
                                                        },
                                                    )),
                                            )
                                            .dropdown_menu(move |menu, _, cx| {
                                                menu.menu_with_icon(
                                                    "Run",
                                                    Icon::primary(cx, IconName::Play),
                                                    Box::new(QueryTabAction::RunSql),
                                                )
                                                .menu_with_icon_and_disabled(
                                                    "Run Selection",
                                                    Icon::primary(cx, IconName::MousePointer2)
                                                        .disabled(is_editor_selected_empty),
                                                    Box::new(QueryTabAction::RunSqlSelection),
                                                    is_editor_selected_empty,
                                                )
                                            }),
                                    )
                                    .child(
                                        Button::new("btn-cancel-query")
                                            .disabled(is_cancel_disabled)
                                            .icon(
                                                Icon::red(cx, IconName::X)
                                                    .disabled(is_cancel_disabled),
                                            )
                                            .small()
                                            .tooltip("Cancel")
                                            .on_click(window.listener_for(
                                                &self.state,
                                                |state, _, _, cx| {
                                                    state.cancel_event(cx);
                                                },
                                            )),
                                    ),
                            ),
                        )
                        .child(Editor::new(&state.editor).context_menu(move |menu, _, cx| {
                            menu.menu_with_icon_and_disabled(
                                "Run",
                                Icon::primary(cx, IconName::Play).disabled(is_run_disabled),
                                Box::new(QueryTabAction::RunSql),
                                is_run_disabled,
                            )
                            .menu_with_icon_and_disabled(
                                "Run Selection",
                                Icon::primary(cx, IconName::MousePointer2)
                                    .disabled(is_editor_selected_empty || is_run_disabled),
                                Box::new(QueryTabAction::RunSqlSelection),
                                is_editor_selected_empty || is_run_disabled,
                            )
                            .separator()
                            .menu("Cut", Box::new(input::Cut))
                            .menu("Copy", Box::new(input::Copy))
                            .menu("Paste", Box::new(input::Paste))
                            .menu("Select All", Box::new(input::SelectAll))
                            .separator()
                            .menu_with_icon_and_disabled(
                                "Format",
                                Icon::primary(cx, IconName::BrushCleaning)
                                    .disabled(is_editor_empty),
                                Box::new(QueryTabAction::FormatSql),
                                is_editor_empty,
                            )
                        }))
                        .on_action(window.listener_for(
                            &self.state,
                            |state, action, window, cx| {
                                state.handle_action(window, cx, action);
                            },
                        )),
                ),
            )
            .child(resizable_panel().when(!state.tables.is_empty(), |this| {
                this.child(
                    v_flex()
                        .size_full()
                        .child(
                            h_flex().id("result-tab-bar").overflow_x_scroll().child(
                                TabBar::new("result-tabs")
                                    .flex_1()
                                    .selected_index(state.active_table)
                                    .on_click(window.listener_for(
                                        &self.state,
                                        |state, idx, _, _| {
                                            state.active_table = *idx;
                                        },
                                    ))
                                    .children(state.tables.iter().enumerate().map(|(idx, _)| {
                                        Tab::new().label(format!("Result #{}", idx + 1))
                                    })),
                            ),
                        )
                        .child(QueryTable::new(state.active_table())),
                )
            }))
    }

    /// Focuses the content in the tab.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
            });
        });
    }

    /// Returns the label for the tab.
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }
}

/// The state used with a [`QueryTab`].
struct QueryTabState {
    /// The id for the active query event.
    active_event: Option<EventId>,

    /// The index of the active table tab.
    active_table: usize,

    /// The state for the editor.
    editor: Entity<EditorState>,

    /// The states for the tables showing query results.
    tables: Vec<Entity<QueryTableState>>,

    /// The subscriptions that handle events originating from the query tables, such as sorting.
    ///
    /// These are saved here since we want to drop the subscriptions when new queries are ran.
    table_subscriptions: Vec<Subscription>,
}

impl QueryTabState {
    /// Creates a new [`QueryTabState`].
    fn new(window: &mut Window, cx: &mut App) -> Self {
        let editor = cx.new(|cx| EditorState::new(window, cx));

        Self {
            active_event: None,
            active_table: 0,
            editor,
            tables: Vec::new(),
            table_subscriptions: Vec::new(),
        }
    }

    /// Handles actions originating from the tab.
    fn handle_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: &QueryTabAction,
    ) {
        match action {
            QueryTabAction::FormatSql => self.format_sql(window, cx),
            QueryTabAction::RunSql => {
                self.clear_results();
                self.run_sql(window, cx, false);
            }
            QueryTabAction::RunSqlSelection => {
                self.clear_results();
                self.run_sql(window, cx, true);
            }
        }
    }

    /// Returns a reference to the active table.
    fn active_table(&self) -> &Entity<QueryTableState> {
        &self.tables[self.active_table]
    }

    /// Cancels the running query event.
    fn cancel_event(&self, cx: &App) {
        let Some(event_id) = self.active_event else {
            return;
        };

        EventManager::cancel(cx, event_id);
    }

    /// Clears the results from the tab.
    fn clear_results(&mut self) {
        self.tables.clear();
        self.table_subscriptions.clear();
        self.active_table = 0;
    }

    /// Formats the SQL within the editor.
    fn format_sql(&self, window: &mut Window, cx: &mut Context<Self>) {
        let formatted = migris::sql::format(&self.editor.read(cx).value(cx));
        self.editor.update(cx, |editor, cx| {
            editor.set_value(window, cx, &formatted);
        });
    }

    /// Triggers an event to run the SQL in the editor.
    fn run_sql(&mut self, window: &mut Window, cx: &mut Context<Self>, selected: bool) {
        let editor = self.editor.read(cx);
        let sql = if selected {
            editor.selected_value(cx)
        } else {
            editor.value(cx)
        };

        let this = cx.entity();
        let event = Event::new(
            RunSqlEvent::new(sql, move |window, cx, result| {
                this.update(cx, |this, cx| {
                    let table = cx.new(|cx| QueryTableState::with_result(window, cx, result));
                    let subscription = cx.subscribe(&table, |_, table, event, cx| {
                        match event {
                            QueryTableEvent::Sort => {
                                // When sorting within a result table on a query tab, we just want to perform the sort in-memory
                                // as attempting to hit the database again for the sorted data would be quite complex.
                                table.update(cx, |table, cx| {
                                    table.sort_data(cx);
                                });
                            }
                        }
                    });

                    this.tables.push(table);
                    this.table_subscriptions.push(subscription);
                    cx.notify();
                });
            })
            .record_history()
            .show_progress(),
        )
        .on_complete({
            let this = cx.entity();
            move |_, cx| {
                this.update(cx, |this, _| {
                    this.active_event = None;
                });
            }
        })
        .on_error(|window, cx, error| {
            notifications::show_error(window, cx, error);
        });

        self.active_event = Some(event.id);
        EventManager::emit(window, cx, event);
    }
}
