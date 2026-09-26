use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window,
    base::{h_flex, v_flex},
    component::{ActiveTheme, Sizable, button::Button},
    div,
};
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::{
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
            .gap_1()
            .child(
                h_flex().pt_1().px_1().justify_end().child(
                    Button::new("table-refresh")
                        .icon(IconName::RefreshCw)
                        .tooltip("Refresh")
                        .small()
                        .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                            state.refresh(window, cx);
                        })),
                ),
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
        let event = Event::new(RunSqlEvent::stream(
            migris::sql::select_all(&self.entity, &self.table.read(cx).order_by(cx)),
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
}
