use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, SharedString,
    Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{ActiveTheme, h_flex, progress::ProgressCircle, v_flex};
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::table::{QueryTable, QueryTableState},
    event::{AppEvent, AppEventKind, EventId, RunSql},
};

const ROW_BATCH_SIZE: usize = 1_000;

const ID_EVENT_DATA_STR: &str = "ID_EVENT_DATA";
const ID_EVENT_DATA: SharedString = SharedString::new_static(ID_EVENT_DATA_STR);

struct TableTabState {
    /// The state for the data table.
    table_state: Option<Entity<QueryTableState>>,
}

impl EventEmitter<AppEvent> for TableTabState {}

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

impl EventEmitter<AppEvent> for TableTab {}

impl TableTab {
    /// Creates a new [`TableTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let state = cx.new(|cx| TableTabState::new(window, cx));
        let _subscription = cx.subscribe(&state, |_, _, event, cx| {
            // Emit the event upwards.
            cx.emit(event.clone());
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
        cx.emit(
            AppEvent::new(AppEventKind::RunSql(RunSql {
                sql: SharedString::from(migris::sql::select_all(&self.entity)),
                show_progress: false,
                stream: true,
            }))
            .with_id(ID_EVENT_DATA),
        );
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
        id: Option<EventId>,
        result: QueryResult,
    ) {
        if let Some(id) = id {
            match id.as_str() {
                ID_EVENT_DATA_STR => {
                    self.state.update(cx, |state, cx| {
                        state.load_data(window, cx, result);
                    });
                }
                _ => {}
            }
        }
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
