use gpui::{AnyElement, App, AppContext, Context, Entity, IntoElement, SharedString, Window};
use migris::Entity as MigrisEntity;

use crate::{
    components::icon::IconName,
    tabs::{query::QueryTab, table::TableTab, view::ViewTab},
};

pub mod query;
pub mod table;
pub mod view;

enum TabState {
    Query(Entity<QueryTab>),
    Table(Entity<TableTab>),
    View(Entity<ViewTab>),
}

pub enum TabVariant {
    Query(usize),
    Table(MigrisEntity),
    View(MigrisEntity),
}

pub struct TabView {
    /// The state for the tab view.
    tab: TabState,

    /// The variant of the tab view.
    variant: TabVariant,
}

impl TabView {
    /// Creates a new [`TabView`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) -> Self {
        let tab = match &variant {
            TabVariant::Query(number) => {
                let tab = cx.new(|cx| QueryTab::new(window, cx, *number));
                TabState::Query(tab)
            }
            TabVariant::Table(entity) => {
                let tab = cx.new(|cx| TableTab::new(window, cx, entity.clone()));
                TabState::Table(tab)
            }
            TabVariant::View(entity) => {
                let tab = cx.new(|cx| ViewTab::new(window, cx, entity.clone()));
                TabState::View(tab)
            }
        };

        Self { tab, variant }
    }

    /// Returns the content for the tab view.
    pub fn content(&self, window: &mut Window, cx: &App) -> AnyElement {
        match &self.tab {
            TabState::Query(tab) => tab.read(cx).content(window, cx).into_any_element(),
            TabState::Table(tab) => tab.read(cx).content(window, cx).into_any_element(),
            TabState::View(tab) => tab.read(cx).content(window, cx).into_any_element(),
        }
    }

    /// Focuses the content in the tab view.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        match &self.tab {
            TabState::Query(tab) => tab.update(cx, |tab, cx| {
                tab.focus(window, cx);
            }),
            TabState::Table(tab) => tab.update(cx, |tab, cx| {
                tab.focus(window, cx);
            }),
            TabState::View(_) => {}
        }
    }

    /// Returns the icon for the tab view.
    pub fn icon(&self) -> IconName {
        match self.variant {
            TabVariant::Query(_) => IconName::Code,
            TabVariant::Table(_) => IconName::Grid3x3,
            TabVariant::View(_) => IconName::Eye,
        }
    }

    /// Returns the label for the tab view.
    pub fn label(&self, cx: &App) -> SharedString {
        match &self.tab {
            TabState::Query(tab) => tab.read(cx).label(),
            TabState::Table(tab) => tab.read(cx).label(),
            TabState::View(tab) => tab.read(cx).label(),
        }
    }

    /// Returns the variant of the tab view.
    pub fn variant(&self) -> &TabVariant {
        &self.variant
    }
}
