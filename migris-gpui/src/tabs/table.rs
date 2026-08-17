use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window,
};
use gpui_component::v_flex;
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::table::{QueryTable, QueryTableEvent, QueryTableState},
    events::{Event, EventManager, RunSqlEvent},
};

/// The state used with a [`TableTab`].
struct TableTabState {
    /// The table entity for the tab.
    entity: MigrisEntity,

    /// The state for the query table.
    table: Entity<QueryTableState>,
}

impl TableTabState {
    /// Creates a new [`TableTabState`].
    fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let table = cx.new(|cx| QueryTableState::new(window, cx));

        cx.subscribe_in(&table, window, |this, _, event, window, cx| match event {
            QueryTableEvent::Sort => this.refresh(window, cx),
        })
        .detach();

        Self { entity, table }
    }

    /// Loads the given table data into the tab.
    fn load_table(&self, cx: &mut App, result: QueryResult) {
        self.table.update(cx, |table, cx| {
            table.init(cx, result);
        });
    }

    /// Refreshes the table data inside the tab.
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

pub struct TableTab {
    /// The state for the table tab.
    state: Entity<TableTabState>,

    /// The table tab label.
    label: SharedString,
}

impl TableTab {
    /// Creates a new [`TableTab`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) -> Self {
        let label = SharedString::from(&entity.name);
        let state = cx.new(|cx| TableTabState::new(window, cx, entity));
        let tab = Self { state, label };
        tab.init(window, cx);
        tab
    }

    /// Initializes the tab.
    fn init(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.refresh(window, cx);
        });
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
