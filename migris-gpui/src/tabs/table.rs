use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window,
    base::{
        Selectable, h_flex,
        input::{InputEvent, InputState},
        v_flex,
    },
    component::{ActiveTheme, Sizable, button::Button, input::Input},
    div,
    prelude::FluentBuilder,
};
use migris::{Entity as MigrisEntity, EntityData, data::QueryResult};

use crate::{
    components::{
        icon::{Icon, IconName},
        table::{QueryTable, QueryTableEvent, QueryTableState},
    },
    events::{Event, EventManager, EventVariant, LoadEntityEvent, RunSqlEvent},
    notifications, shared,
};

pub struct TableTab {
    /// The state for the table tab.
    state: Entity<TableTabState>,

    /// The table tab label.
    label: SharedString,
}

impl TableTab {
    /// Creates a new [`TableTab`].
    pub fn new(window: &mut Window, cx: &mut App, entity: MigrisEntity) -> Self {
        let label = SharedString::from(&entity.name);
        let state = cx.new(|cx| TableTabState::new(window, cx, entity));
        Self { state, label }
    }

    /// Performs any needed behavior for closing the tab.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.kill_process(window, cx);
        });
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
                                        .small()
                                        .icon(IconName::Funnel)
                                        .selected(state.show_filter)
                                        .tooltip("Filter")
                                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                            state.toggle_filter(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("btn-refresh")
                                        .small()
                                        .icon(IconName::RefreshCw)
                                        .tooltip("Refresh")
                                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                            state.refresh(window, cx);
                                        })),
                                ),
                        ),
                    )
                    .when(state.show_filter, |this| {
                        this.child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Input::new(&state.filter_input)
                                        .small()
                                        .cleanable(true)
                                        .prefix(Icon::new(cx, IconName::Funnel)),
                                )
                                .child(
                                    Button::new("btn-apply")
                                        .small()
                                        .icon(IconName::Play)
                                        .tooltip("Apply")
                                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                            state.refresh_data(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("btn-toggle-multi")
                                        .small()
                                        .icon(IconName::Columns3)
                                        .selected(state.enable_multi_filter)
                                        .tooltip(if state.enable_multi_filter {
                                            "Disable Multi-Column"
                                        } else {
                                            "Enable Multi-Column"
                                        })
                                        .on_click(window.listener_for(&self.state, |state, _, _, _| {
                                            state.enable_multi_filter = !state.enable_multi_filter;
                                        })),
                                )
                                .child(
                                    Button::new("btn-clear-filter")
                                        .small()
                                        .icon(IconName::FunnelX)
                                        .tooltip("Clear Filter")
                                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                            state.clear_filter(window, cx);
                                        })),
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

/// The state used with a [`TableTab`].
struct TableTabState {
    /// The entity being shown in the tab.
    entity: MigrisEntity,

    /// The data for the entity.
    data: Option<EntityData>,

    /// Whether to enable multi-column filtering.
    enable_multi_filter: bool,

    /// The state for the filter input.
    filter_input: Entity<InputState>,

    /// Whether to show the filter elements.
    show_filter: bool,

    /// The state for the query table.
    table: Entity<QueryTableState>,
}

impl TableTabState {
    /// Creates a new [`TableTabState`].
    fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder(shared::FILTER_PLACEHOLDER));
        cx.subscribe_in(&filter_input, window, |this, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.refresh_data(window, cx);
            }
        })
        .detach();

        let table = cx.new(|cx| QueryTableState::new(window, cx));
        cx.subscribe_in(&table, window, |this, _, event, window, cx| match event {
            QueryTableEvent::Sort => this.refresh_data(window, cx),
        })
        .detach();

        // Defer the initial refresh of the tab's data.
        cx.defer_in(window, |this, window, cx| {
            this.refresh(window, cx);
        });

        Self {
            entity,
            data: None,
            enable_multi_filter: false,
            filter_input,
            show_filter: false,
            table,
        }
    }

    /// Clears any active filter.
    fn clear_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter_input.update(cx, |filter_input, cx| {
            filter_input.clean(window, cx);
        });

        self.refresh_data(window, cx);
    }

    /// Kills the process associated with the tab's result stream.
    fn kill_process(&self, window: &mut Window, cx: &mut App) {
        let process_id = self.table.read(cx).process_id(cx);
        let event = Event::new(EventVariant::KillProcess(process_id));
        EventManager::emit(window, cx, event);
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
        self.refresh_entity(window, cx);
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

    /// Refreshes the entity data inside the tab.
    fn refresh_entity(&self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.entity();
        let event = Event::new(LoadEntityEvent::new(self.entity.clone(), move |_, cx, data| {
            this.update(cx, |this, cx| {
                this.table.update(cx, |table, cx| {
                    let EntityData::Table(data) = &data else {
                        return;
                    };

                    table.build_column_index_map(cx, data.indexes());
                });

                this.data = Some(data);
            });
        }))
        .on_error(|window, cx, error| {
            notifications::show_error(window, cx, error);
        });

        EventManager::emit(window, cx, event);
    }

    /// Toggles the visibility of the filter elements.
    fn toggle_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_filter = !self.show_filter;

        if self.show_filter {
            self.filter_input.update(cx, |filter_input, cx| {
                filter_input.focus(window, cx);
            });
        }
    }

    /// Returns the WHERE clause built from the content within the filter input.
    fn where_clause(&self, cx: &App) -> String {
        let filter = self.filter_input.read(cx).value();
        if filter.trim().is_empty() {
            String::new()
        } else if self.enable_multi_filter {
            let filter = filter
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
                .replace('\'', "''");
            let filters: Vec<String> = self
                .table
                .read(cx)
                .columns(cx)
                .iter()
                .map(|column| format!("`{}` LIKE '%{}%'", column.name, filter))
                .collect();

            format!("WHERE {}", filters.join(" OR "))
        } else {
            format!("WHERE {}", filter)
        }
    }
}
