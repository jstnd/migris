use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window,
};
use gpui_component::v_flex;
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::table::{QueryTable, QueryTableState},
    events::{Event, EventManager, RunSqlEvent},
};

/// The state used with a [`TableTab`].
struct TableTabState {
    /// The state for the data table.
    table: Entity<QueryTableState>,
}

impl TableTabState {
    /// Creates a new [`TableTabState`].
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table = cx.new(|cx| QueryTableState::new(window, cx));
        Self { table }
    }

    /// Loads the given table data into the tab.
    fn load_table(&self, cx: &mut Context<Self>, result: QueryResult) {
        self.table.update(cx, |table, cx| {
            table.init(cx, result);
        });
    }
}

pub struct TableTab {
    /// The state for the table tab.
    state: Entity<TableTabState>,

    /// The table entity for the tab.
    entity: MigrisEntity,

    /// The table tab label.
    label: SharedString,
}

impl TableTab {
    /// Creates a new [`TableTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let state = cx.new(|cx| TableTabState::new(window, cx));
        let label = SharedString::from(&entity.name);
        let tab = Self {
            state,
            entity,
            label,
        };

        tab.init(window, cx);
        tab
    }

    /// Initializes the tab.
    ///
    /// This will emit the events needed to retrieve the data for the tab.
    fn init(&self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let event = RunSqlEvent::stream(migris::sql::select_all(&self.entity)).on_result(
            move |result, _, cx| {
                state.update(cx, |state, cx| {
                    state.load_table(cx, result);
                });
            },
        );

        EventManager::emit(window, cx, Event::new(event));
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
            .child(QueryTable::new(&state.table))
    }
}
