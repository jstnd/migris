use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, SharedString,
    Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{ActiveTheme, h_flex, progress::ProgressCircle, v_flex};
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::table::{QueryTable, QueryTableState},
    events::{Event, EventId, EventManager, RunSqlEvent},
};

const ROW_BATCH_SIZE: usize = 1_000;

struct TableTabState {
    /// The state for the data table.
    table_state: Option<Entity<QueryTableState>>,
}

impl EventEmitter<EventId> for TableTabState {}

impl TableTabState {
    /// Creates a new [`TableTabState`].
    fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self { table_state: None }
    }

    /// Loads the given table data into the tab.
    fn load_data(&mut self, window: &mut Window, cx: &mut Context<Self>, result: QueryResult) {
        let table_state = cx.new(|cx| {
            let mut state = QueryTableState::new(window, cx, result);
            state.load(cx, ROW_BATCH_SIZE);
            state
        });

        self.table_state = Some(table_state);
    }
}

pub struct TableTab {
    /// The state for the table tab.
    state: Entity<TableTabState>,

    /// The table entity for the tab.
    entity: MigrisEntity,

    /// The table tab label.
    label: SharedString,

    /// The subscription for the tab.
    ///
    /// This will mainly be used for emitting events from the tab upwards.
    _subscription: Subscription,
}

impl EventEmitter<EventId> for TableTab {}

impl TableTab {
    /// Creates a new [`TableTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let state = cx.new(|cx| TableTabState::new(window, cx));
        let _subscription = cx.subscribe(&state, |_, _, event, cx| {
            // Emit the event upwards.
            cx.emit(*event);
        });

        let label = SharedString::from(&entity.name);

        Self {
            state,
            entity,
            label,
            _subscription,
        }
    }

    /// Initializes the tab.
    ///
    /// This will emit the events needed to retrieve the data for the tab.
    pub fn init(&self, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let event = RunSqlEvent::stream(migris::sql::select_all(&self.entity)).on_result(
            move |result, window, cx| {
                state.update(cx, |state, cx| {
                    state.load_data(window, cx, result);
                });
            },
        );

        EventManager::emit(cx, Event::new(event));
    }

    /// Returns the label for the tab.
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }

    /// Returns the content for the tab.
    pub fn content(&self, _: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);

        v_flex()
            .gap_1()
            .size_full()
            .when(state.table_state.is_none(), |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .child(
                            ProgressCircle::new("table-loading")
                                .color(cx.theme().primary)
                                .loading(true),
                        )
                        .child("Loading..."),
                )
            })
            .when_some(state.table_state.as_ref(), |this, table_state| {
                this.child(QueryTable::new(table_state))
            })
    }
}
