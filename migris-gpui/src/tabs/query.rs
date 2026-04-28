use gpui::{
    Action, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable,
    button::{Button, DropdownButton},
    h_flex, input,
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
    events::{Event, EventManager, RunSqlEvent},
};

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
pub enum QueryTabAction {
    RunSql,
    RunSqlSelection,
}

struct QueryTabState {
    /// The state for the editor.
    editor: Entity<EditorState>,

    /// The states for the tables showing query results.
    tables: Vec<Entity<QueryTableState>>,

    /// The index of the active table tab.
    active_table: usize,
}

impl QueryTabState {
    /// Creates a new [`QueryTabState`].
    fn new(window: &mut Window, cx: &mut App) -> Self {
        let editor = cx.new(|cx| EditorState::new(window, cx));

        Self {
            editor,
            tables: Vec::new(),
            active_table: 0,
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

    /// Clears the results from the tab.
    fn clear_results(&mut self) {
        self.tables.clear();
        self.active_table = 0;
    }

    /// Triggers an event to run the SQL in the editor.
    fn run_sql(&self, window: &mut Window, cx: &mut Context<Self>, selected: bool) {
        let editor = self.editor.read(cx);
        let sql = if selected {
            editor.selected_value(cx)
        } else {
            editor.value(cx)
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

        EventManager::emit(window, cx, Event::new(event));
    }
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

    /// Returns the label for the tab.
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);
        let is_editor_empty = state.editor.read(cx).is_empty(cx);
        let is_editor_selected_empty = state.editor.read(cx).selected_value(cx).is_empty();

        v_resizable(format!("query-tab-{}", self.number))
            .child(
                resizable_panel().child(
                    v_flex()
                        .gap_1()
                        .pt_1()
                        .size_full()
                        .on_action(
                            window.listener_for(&self.state, |state, action, window, cx| {
                                state.handle_action(window, cx, action);
                            }),
                        )
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
                                            Icon::new(cx, IconName::Play).primary(cx),
                                            Box::new(QueryTabAction::RunSql),
                                        )
                                        .menu_with_icon_and_disabled(
                                            "Run Selection",
                                            Icon::new(cx, IconName::MousePointer2)
                                                .disabled(is_editor_selected_empty)
                                                .primary(cx),
                                            Box::new(QueryTabAction::RunSqlSelection),
                                            is_editor_selected_empty,
                                        )
                                    }),
                            ),
                        )
                        .child(
                            Editor::new(&state.editor).context_menu(move |menu, _, cx| {
                                menu.menu_with_icon_and_disabled(
                                    "Run",
                                    Icon::new(cx, IconName::Play)
                                        .disabled(is_editor_empty)
                                        .primary(cx),
                                    Box::new(QueryTabAction::RunSql),
                                    is_editor_empty,
                                )
                                .menu_with_icon_and_disabled(
                                    "Run Selection",
                                    Icon::new(cx, IconName::MousePointer2)
                                        .disabled(is_editor_selected_empty)
                                        .primary(cx),
                                    Box::new(QueryTabAction::RunSqlSelection),
                                    is_editor_selected_empty,
                                )
                                .separator()
                                .menu("Cut", Box::new(input::Cut))
                                .menu("Copy", Box::new(input::Copy))
                                .menu("Paste", Box::new(input::Paste))
                                .menu("Select All", Box::new(input::SelectAll))
                            }),
                        ),
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
