use std::{ffi::OsStr, path::PathBuf, time::Duration};

use anyhow::Result;
use gpui_kit::{
    Action, Anchor, App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyBinding,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Task, Window,
    base::{Disableable, h_flex, resizable_panel, v_flex, v_resizable},
    component::{
        ActiveTheme, Sizable,
        button::{Button, DropdownButton},
        input,
        progress::ProgressCircle,
        tab::{Tab, TabBar},
    },
    div,
    prelude::FluentBuilder,
};
use notify_debouncer_mini::notify::RecursiveMode;

use crate::{
    components::{
        editor::{Editor, EditorEvent, EditorState},
        icon::{Icon, IconName},
        table::{QueryTable, QueryTableEvent, QueryTableState},
    },
    events::{Event, EventId, EventManager, EventVariant, RunSqlEvent},
    notifications,
    progress::QueryProgress,
};

const KEY_CONTEXT: &str = "QUERY_TAB";

// This is the key context that gpui-kit's Input component uses.
const INPUT_CONTEXT: &str = "Input";

/// Initializes configuration for the query tab.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-shift-f", QueryTabAction::FormatSql, Some(KEY_CONTEXT)),
        KeyBinding::new("secondary-shift-f", QueryTabAction::FormatSql, Some(INPUT_CONTEXT)),
        KeyBinding::new("secondary-enter", QueryTabAction::RunSql, Some(KEY_CONTEXT)),
        KeyBinding::new("secondary-enter", QueryTabAction::RunSql, Some(INPUT_CONTEXT)),
        KeyBinding::new(
            "secondary-shift-enter",
            QueryTabAction::RunSqlSelection,
            Some(KEY_CONTEXT),
        ),
        KeyBinding::new(
            "secondary-shift-enter",
            QueryTabAction::RunSqlSelection,
            Some(INPUT_CONTEXT),
        ),
    ]);
}

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

    /// The query tab label used when the tab has pending changes.
    label_changes: SharedString,

    /// The query tab number.
    number: usize,
}

impl QueryTab {
    /// Creates a new [`QueryTab`].
    pub fn new(window: &mut Window, cx: &mut App, number: usize) -> Self {
        Self {
            state: cx.new(|cx| QueryTabState::new(window, cx)),
            label: SharedString::from(format!("Query #{}", number)),
            label_changes: SharedString::from(format!("Query #{}*", number)),
            number,
        }
    }

    /// Creates a new [`QueryTab`] with the given path.
    pub fn with_path(window: &mut Window, cx: &mut App, number: usize, path: PathBuf) -> Self {
        let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        let label = SharedString::from(file_name);
        let label_changes = SharedString::from(format!("{}*", file_name));

        Self {
            state: cx.new(|cx| QueryTabState::with_path(window, cx, path)),
            label,
            label_changes,
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
        let is_editor_selected_empty = state.editor.read(cx).is_selected_empty(cx);
        let has_active_event = state.active_event.is_some();

        let is_run_disabled = is_editor_empty || has_active_event;
        let is_cancel_disabled = !has_active_event;

        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&state.focus_handle)
            .size_full()
            .child(
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
                                                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, cx| {
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
                                                    .icon(Icon::red(cx, IconName::X).disabled(is_cancel_disabled))
                                                    .small()
                                                    .tooltip("Cancel")
                                                    .on_click(window.listener_for(&self.state, |state, _, _, cx| {
                                                        state.cancel_event(cx);
                                                    })),
                                            )
                                            .when_some(state.query_progress.as_ref(), |this, progress| {
                                                this.child(
                                                    h_flex()
                                                        .gap_1()
                                                        .ml_1()
                                                        .child(
                                                            ProgressCircle::new("query-progress")
                                                                .color(cx.theme().primary)
                                                                .value(progress.value()),
                                                        )
                                                        .child(div().text_sm().child(progress.label())),
                                                )
                                            }),
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
                                        Icon::primary(cx, IconName::BrushCleaning).disabled(is_editor_empty),
                                        Box::new(QueryTabAction::FormatSql),
                                        is_editor_empty,
                                    )
                                })),
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
                                            .on_click(window.listener_for(&self.state, |state, idx, _, _| {
                                                state.active_table = *idx;
                                            }))
                                            .children(
                                                state
                                                    .tables
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(idx, _)| Tab::new().label(format!("Result #{}", idx + 1))),
                                            ),
                                    ),
                                )
                                .child(QueryTable::new(state.active_table())),
                        )
                    })),
            )
            .on_action(window.listener_for(&self.state, |state, action, window, cx| {
                state.handle_action(window, cx, action);
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
    pub fn label(&self, cx: &App) -> SharedString {
        if self.state.read(cx).has_changes {
            self.label_changes.clone()
        } else {
            self.label.clone()
        }
    }

    /// Saves the content of the tab.
    pub fn save(&self, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.save(cx);
        });
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

    /// The focus handle for the tab content.
    focus_handle: FocusHandle,

    /// Whether the editor has changes.
    has_changes: bool,

    /// The saved file path for the editor content.
    path: Option<PathBuf>,

    /// Whether the file at the saved path has been externally modified.
    path_modified: bool,

    /// The task for watching the saved path for external modifications.
    _path_watcher: Option<Task<Result<()>>>,

    /// The progress for the running query.
    query_progress: Option<QueryProgress>,

    /// The task for updating the timer associated with query progress.
    query_progress_task: Option<Task<()>>,

    /// Whether to suppress the next path modified event.
    suppress_path_modified: bool,

    /// Whether to suppress the path removed notification.
    suppress_path_removed: bool,

    /// The states for the tables showing query results.
    tables: Vec<Entity<QueryTableState>>,
}

impl QueryTabState {
    /// Creates a new [`QueryTabState`].
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| EditorState::new(window, cx));
        cx.subscribe(&editor, |this, _, event, _| match event {
            EditorEvent::Change => {
                // Only track changes if we have a saved path.
                if this.path.is_some() {
                    this.has_changes = true;
                }
            }
        })
        .detach();

        let focus_handle = cx.focus_handle();
        cx.on_focus_in(&focus_handle, window, |this, window, cx| {
            // Check that the saved path still exists when refocusing the tab content.
            if let Some(path) = &this.path
                && !path.exists()
                && !this.suppress_path_removed
            {
                this.path_modified = true;
            }

            this.handle_path_modified(window, cx);
        })
        .detach();

        Self {
            active_event: None,
            active_table: 0,
            editor,
            focus_handle,
            has_changes: false,
            path: None,
            path_modified: false,
            _path_watcher: None,
            query_progress: None,
            query_progress_task: None,
            suppress_path_modified: false,
            suppress_path_removed: false,
            tables: Vec::new(),
        }
    }

    /// Creates a new [`QueryTabState`] with the given path.
    fn with_path(window: &mut Window, cx: &mut Context<Self>, path: PathBuf) -> Self {
        let path_watcher = cx.spawn({
            let path = path.clone();
            async move |this, cx| {
                let (tx, mut rx) = tokio::sync::mpsc::channel(1);
                let mut debouncer = notify_debouncer_mini::new_debouncer(Duration::from_millis(100), move |result| {
                    _ = tx.blocking_send(result);
                })?;
                debouncer.watcher().watch(&path, RecursiveMode::NonRecursive)?;

                while let Some(result) = rx.recv().await {
                    if result.is_ok() {
                        _ = this.update_in(cx, |this, window, cx| {
                            this.path_modified = true;
                            this.handle_path_modified(window, cx);
                        });
                    }
                }
                Ok(())
            }
        });

        let mut state = Self::new(window, cx);
        state.path = Some(path);
        state._path_watcher = Some(path_watcher);
        state.load_path(window, cx);
        state
    }

    /// Handles actions originating from the tab.
    fn handle_action(&mut self, window: &mut Window, cx: &mut Context<Self>, action: &QueryTabAction) {
        match action {
            QueryTabAction::FormatSql => self.format_sql(window, cx),
            QueryTabAction::RunSql => {
                if self.editor.read(cx).is_empty(cx) {
                    return;
                }

                self.clear_results();
                self.run_sql(window, cx, false);
            }
            QueryTabAction::RunSqlSelection => {
                if self.editor.read(cx).is_selected_empty(cx) {
                    return;
                }

                self.clear_results();
                self.run_sql(window, cx, true);
            }
        }
    }

    /// Handles notification display for external path modifications.
    fn handle_path_modified(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.path_modified || !self.focus_handle.contains_focused(window, cx) {
            return;
        }

        if self.suppress_path_modified {
            self.path_modified = false;
            self.suppress_path_modified = false;
            return;
        }

        let Some(path) = &self.path else {
            return;
        };

        let this = cx.entity();
        if path.exists() {
            notifications::show_confirmation(
                window,
                cx,
                "File Modified",
                "This file has been modified by another program. Do you want to reload it?",
                {
                    let this = this.clone();
                    move |_, cx| {
                        this.update(cx, |this, _| {
                            this.has_changes = true;
                            this.path_modified = false;
                        });
                    }
                },
                {
                    let this = this.clone();
                    move |window, cx| {
                        this.update(cx, |this, cx| {
                            this.path_modified = false;
                            this.load_path(window, cx);
                        });
                    }
                },
            );
        } else {
            notifications::show_confirmation(
                window,
                cx,
                "File Removed",
                "This file doesn't exist anymore. Do you want to keep this tab open?",
                move |window, cx| {
                    EventManager::emit(window, cx, EventVariant::CloseActiveTab);
                },
                {
                    let this = this.clone();
                    move |_, cx| {
                        this.update(cx, |this, _| {
                            this.has_changes = true;
                            this.path_modified = false;
                            this.suppress_path_removed = true;
                        });
                    }
                },
            );
        }
    }

    /// Returns a reference to the active table.
    fn active_table(&self) -> &Entity<QueryTableState> {
        &self.tables[self.active_table]
    }

    /// Cancels the running query event.
    fn cancel_event(&self, cx: &App) {
        if let Some(event_id) = self.active_event {
            EventManager::cancel(cx, event_id);
        }
    }

    /// Clears the results from the tab.
    fn clear_results(&mut self) {
        self.tables.clear();
        self.active_table = 0;
    }

    /// Formats the SQL within the editor.
    fn format_sql(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let formatted = migris::sql::format(&self.editor.read(cx).value(cx));
        self.has_changes = true;
        self.editor.update(cx, |editor, cx| {
            editor.set_value(window, cx, &formatted);
        });
    }

    /// Loads the editor with the content from the saved path.
    fn load_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = &self.path else {
            return;
        };

        // TODO: keep content if path doesn't exist
        let content = std::fs::read_to_string(path).unwrap_or_default();
        self.editor.update(cx, |editor, cx| {
            editor.set_value(window, cx, &content);
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
                    cx.subscribe_in(&table, window, |_, table, event, window, cx| {
                        match event {
                            QueryTableEvent::OpenImage(image) => {
                                let event = Event::new(EventVariant::OpenImage(image.clone()));
                                EventManager::emit(window, cx, event);
                            }
                            QueryTableEvent::Sort => {
                                // When sorting within a result table on a query tab, we just want to perform the sort in-memory
                                // as attempting to hit the database again for the sorted data would be quite complex.
                                table.update(cx, |table, cx| {
                                    table.sort_data(cx);
                                });
                            }
                        }
                    })
                    .detach();

                    this.tables.push(table);
                    cx.notify();
                });
            })
            .record_history()
            .on_progress({
                let this = cx.entity();
                move |_, cx, complete, total| {
                    this.update(cx, |this, cx| {
                        if let Some(progress) = &mut this.query_progress {
                            progress.update(complete);
                        } else {
                            let mut progress = QueryProgress::new(total);
                            progress.update(complete);
                            this.query_progress = Some(progress);
                            this.query_progress_task = Some(cx.spawn(async |this, cx| {
                                loop {
                                    // Update query progress timer every 100 milliseconds.
                                    cx.background_executor().timer(Duration::from_millis(100)).await;
                                    let running = this
                                        .update(cx, |this, cx| {
                                            if this.query_progress.is_none() {
                                                return false;
                                            }

                                            cx.notify();
                                            true
                                        })
                                        .unwrap_or(false);

                                    if !running {
                                        break;
                                    }
                                }
                            }));
                        }
                    });
                }
            }),
        )
        .on_complete({
            let this = cx.entity();
            move |_, cx| {
                this.update(cx, |this, _| {
                    this.active_event = None;
                    this.query_progress = None;
                    this.query_progress_task = None;
                });
            }
        })
        .on_error(|window, cx, error| {
            notifications::show_error(window, cx, error);
        });

        self.active_event = Some(event.id);
        EventManager::emit(window, cx, event);
    }

    /// Saves the content of the tab.
    fn save(&mut self, cx: &App) {
        let Some(path) = &self.path else {
            return;
        };

        // TODO: log errors
        let content = self.editor.read(cx).value(cx);
        if let Some(parent) = path.parent() {
            _ = std::fs::create_dir_all(parent);
        }
        _ = std::fs::write(path, content.as_str());

        self.has_changes = false;
        self.suppress_path_modified = true;
        self.suppress_path_removed = false;
    }
}
