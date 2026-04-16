use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, IntoElement, SharedString,
    Subscription, Window,
};
use migris::{Entity as MigrisEntity, data::QueryResult};

use crate::{
    components::icon::IconName,
    event::{AppEvent, EventId},
    tabs::{query::QueryTab, table::TableTab},
};

pub mod query;
pub mod table;

pub enum TabKind {
    Query(usize),
    Table(MigrisEntity),
}

enum TabState {
    Query(Entity<QueryTab>),
    Table(Entity<TableTab>),
}

pub struct TabView {
    /// The kind for the tab view.
    kind: TabKind,

    /// The state for the tab view.
    tab: TabState,

    /// The subscription for the tab view.
    ///
    /// This will mainly be used for emitting events from the tab upwards.
    _subscription: Subscription,
}

impl EventEmitter<AppEvent> for TabView {}

impl TabView {
    /// Creates a new [`TabView`] of the given kind.
    pub fn new(window: &mut Window, cx: &mut Context<Self>, kind: TabKind) -> Self {
        let tab = match &kind {
            TabKind::Query(number) => {
                let tab = cx.new(|cx| QueryTab::new(window, cx, *number));
                TabState::Query(tab)
            }
            TabKind::Table(entity) => {
                let tab = cx.new(|cx| TableTab::new(window, cx, entity.clone()));
                TabState::Table(tab)
            }
        };

        // Create subscription that will emit events upwards.
        let _subscription = match &tab {
            TabState::Query(tab) => cx.subscribe(tab, |_, _, event, cx| {
                cx.emit(event.clone());
            }),
            TabState::Table(tab) => cx.subscribe(tab, |_, _, event, cx| {
                cx.emit(event.clone());
            }),
        };

        // Initialize the tab if needed.
        match &tab {
            TabState::Query(_) => {}
            TabState::Table(tab) => {
                tab.update(cx, |tab, cx| {
                    tab.init(cx);
                });
            }
        }

        Self {
            kind,
            tab,
            _subscription,
        }
    }

    /// Returns the content for the tab view.
    pub fn content(&self, window: &mut Window, cx: &App) -> AnyElement {
        match &self.tab {
            TabState::Query(tab) => tab.read(cx).content(window, cx).into_any_element(),
            TabState::Table(tab) => tab.read(cx).content(window, cx).into_any_element(),
        }
    }

    /// Returns the icon for the tab view.
    pub fn icon(&self) -> IconName {
        match self.kind {
            TabKind::Query(_) => IconName::Code,
            TabKind::Table(_) => IconName::Grid3x3,
        }
    }

    /// Returns the kind for the tab view.
    pub fn kind(&self) -> &TabKind {
        &self.kind
    }

    /// Returns the label for the tab view.
    pub fn label(&self, cx: &App) -> SharedString {
        match &self.tab {
            TabState::Query(tab) => tab.read(cx).label(),
            TabState::Table(tab) => tab.read(cx).label(),
        }
    }

    /// Loads the given query result into the tab.
    pub fn load_result(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        id: Option<EventId>,
        result: QueryResult,
    ) {
        match &self.tab {
            TabState::Query(tab) => {
                tab.update(cx, |tab, cx| {
                    tab.load_result(window, cx, result);
                });
            }
            TabState::Table(tab) => {
                tab.update(cx, |tab, cx| {
                    tab.load_result(window, cx, id, result);
                });
            }
        }
    }
}
