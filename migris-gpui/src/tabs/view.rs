use gpui::{App, AppContext, Context, Entity, IntoElement, SharedString, Window};
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::table::{QueryTable, QueryTableEvent, QueryTableState},
    events::{Event, EventManager, RunSqlEvent},
};

pub struct ViewTab {
    /// The state for the tab.
    state: Entity<ViewTabState>,

    /// The tab label.
    label: SharedString,
}

impl ViewTab {
    /// Creates a new [`ViewTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let label = SharedString::from(&entity.name);
        let state = cx.new(|cx| ViewTabState::new(window, cx, entity));
        state.update(cx, |state, cx| {
            state.refresh(window, cx);
        });

        Self { state, label }
    }

    /// Returns the content for the tab.
    pub fn content(&self, _: &mut Window, cx: &App) -> impl IntoElement {
        let state = self.state.read(cx);

        QueryTable::new(&state.table)
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

    /// The state for the query table.
    table: Entity<QueryTableState>,
}

impl ViewTabState {
    /// Creates a new [`ViewTabState`].
    fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let table = cx.new(|cx| QueryTableState::new(window, cx));
        cx.subscribe_in(&table, window, |this, _, event, window, cx| match event {
            QueryTableEvent::Sort => this.refresh(window, cx),
        })
        .detach();

        Self { entity, table }
    }

    /// Loads the given query result into the tab.
    fn load_table(&self, cx: &mut App, result: QueryResult) {
        self.table.update(cx, |table, cx| {
            table.init(cx, result);
        });
    }

    /// Refreshes the data inside the tab.
    fn refresh(&self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.entity();
        let event = RunSqlEvent::stream(migris::sql::select_all(
            &self.entity,
            &self.table.read(cx).order_by(cx),
        ))
        .on_result(move |result, _, cx| {
            this.update(cx, |this, cx| {
                this.load_table(cx, result);
            });
        });

        EventManager::emit(window, cx, Event::new(event));
    }
}
