use gpui::{AnyElement, App, AppContext, Context, Entity, IntoElement, SharedString, Window};
use migris::Entity as MigrisEntity;

use crate::{
    components::icon::IconName,
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
}

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

        Self { kind, tab }
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
}
