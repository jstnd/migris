use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window,
    base::{
        h_flex,
        input::{InputEvent, InputState},
        v_flex,
    },
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonVariants},
        input::Input,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::{
        editor::{Editor, EditorState},
        icon::IconName,
        table::{QueryTable, QueryTableEvent, QueryTableState},
    },
    events::{Event, EventManager, RunSqlEvent},
    notifications,
};

pub struct ViewTab {
    /// The state for the tab.
    state: Entity<ViewTabState>,

    /// The tab label.
    label: SharedString,
}

impl ViewTab {
    /// Creates a new [`ViewTab`].
    pub fn new(window: &mut Window, cx: &mut App, entity: MigrisEntity) -> Self {
        let label = SharedString::from(&entity.name);
        let state = cx.new(|cx| ViewTabState::new(window, cx, entity));
        state.update(cx, |state, cx| {
            state.refresh(window, cx);
        });

        Self { state, label }
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);

        v_flex()
            .size_full()
            .child(
                v_flex()
                    .gap_1()
                    .p_1()
                    .child(
                        h_flex().justify_end().child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Button::new("btn-filter")
                                        .icon(IconName::Funnel)
                                        .tooltip("Filter")
                                        .small()
                                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                            state.toggle_filter(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("btn-refresh")
                                        .icon(IconName::RefreshCw)
                                        .tooltip("Refresh")
                                        .small()
                                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                            state.refresh(window, cx);
                                        })),
                                ),
                        ),
                    )
                    .when(state.show_filter, |this| {
                        this.child(
                            h_flex()
                                .h(px(68.0))
                                .gap_1()
                                .child(Editor::new(&state.filter_editor).bordered(true))
                                .child(
                                    v_flex()
                                        .w_1_5()
                                        .gap_1()
                                        .child(Input::new(&state.filter_multicolumn_input).cleanable(true))
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    Button::new("btn-apply-filter")
                                                        .flex_1()
                                                        .label("Apply")
                                                        .primary()
                                                        .on_click(window.listener_for(
                                                            &self.state,
                                                            |state, _, window, cx| {
                                                                state.refresh_data(window, cx);
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new("btn-clear-filter").flex_1().label("Clear").on_click(
                                                        window.listener_for(&self.state, |state, _, window, cx| {
                                                            state.clear_filter(window, cx);
                                                        }),
                                                    ),
                                                ),
                                        ),
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .size_full()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(QueryTable::new(&state.table)),
            )
    }

    /// Focuses the content in the tab.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.table.update(cx, |table, cx| {
                table.focus(window, cx);
            });
        });
    }

    /// Returns the label for the tab.
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }
}

/// The state used with a [`ViewTab`].
struct ViewTabState {
    /// The entity being shown in the tab.
    entity: MigrisEntity,

    /// The state for the filter editor.
    filter_editor: Entity<EditorState>,

    /// The state for the multi-column filter input.
    filter_multicolumn_input: Entity<InputState>,

    /// Whether to show the filter elements.
    show_filter: bool,

    /// The state for the query table.
    table: Entity<QueryTableState>,
}

impl ViewTabState {
    /// Creates a new [`ViewTabState`].
    fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let filter_multicolumn_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter multi-column filter..."));
        cx.subscribe_in(
            &filter_multicolumn_input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.set_multicolumn_filter(window, cx),
                InputEvent::PressEnter { .. } => this.refresh_data(window, cx),
                _ => {}
            },
        )
        .detach();

        let table = cx.new(|cx| QueryTableState::new(window, cx));
        cx.subscribe_in(&table, window, |this, _, event, window, cx| match event {
            QueryTableEvent::Sort => this.refresh_data(window, cx),
        })
        .detach();

        Self {
            entity,
            filter_editor: cx.new(|cx| EditorState::new(window, cx)),
            filter_multicolumn_input,
            show_filter: false,
            table,
        }
    }

    /// Clears any active filter.
    fn clear_filter(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter_editor.update(cx, |filter_editor, cx| {
            filter_editor.clear(window, cx);
        });
        self.filter_multicolumn_input.update(cx, |input, cx| {
            input.clean(window, cx);
        });

        self.refresh_data(window, cx);
    }

    /// Loads the given query result into the tab.
    fn load_table(&self, cx: &mut App, result: QueryResult) {
        self.table.update(cx, |table, cx| {
            table.init(cx, result);
        });
    }

    /// Refreshes all content inside the tab.
    fn refresh(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_data(window, cx);
    }

    /// Refreshes the table data inside the tab.
    fn refresh_data(&self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.entity();
        let event = Event::new(RunSqlEvent::stream(
            migris::sql::select_all(&self.entity, &self.where_clause(cx), &self.table.read(cx).order_by(cx)),
            move |_, cx, result| {
                this.update(cx, |this, cx| {
                    this.load_table(cx, result);
                });
            },
        ))
        .on_error(|window, cx, error| {
            notifications::show_error(window, cx, error);
        });

        EventManager::emit(window, cx, event);
    }

    /// Sets the filter built from the content within the multi-column filter input.
    fn set_multicolumn_filter(&self, window: &mut Window, cx: &mut Context<Self>) {
        let filter = self.filter_multicolumn_input.read(cx).value();
        if filter.is_empty() {
            self.filter_editor.update(cx, |filter_editor, cx| {
                filter_editor.clear(window, cx);
            });
            return;
        }

        let filter = filter.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let filters: Vec<String> = self
            .table
            .read(cx)
            .columns(cx)
            .iter()
            .map(|column| format!("`{}` LIKE '%{}%'", column.name, filter))
            .collect();

        self.filter_editor.update(cx, |filter_editor, cx| {
            filter_editor.set_value(window, cx, &filters.join(" OR "));
        });
    }

    /// Toggles the visibility of the filter elements.
    fn toggle_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_filter = !self.show_filter;

        if self.show_filter {
            self.filter_editor.update(cx, |filter_editor, cx| {
                filter_editor.focus(window, cx);
            })
        }
    }

    /// Returns the WHERE clause built from the content within the filter editor.
    fn where_clause(&self, cx: &App) -> String {
        let filter_editor = self.filter_editor.read(cx);
        if filter_editor.is_empty(cx) {
            String::new()
        } else {
            format!("WHERE {}", filter_editor.value(cx))
        }
    }
}
