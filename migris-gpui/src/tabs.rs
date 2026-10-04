use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window};
use migris::Entity as MigrisEntity;

use crate::{
    components::icon::IconName,
    tabs::{query::QueryTab, table::TableTab, view::ViewTab},
};

pub mod query;
pub mod table;
pub mod view;

/// Initializes configuration for tabs.
pub fn init(cx: &mut App) {
    query::init(cx);
}

enum TabState {
    Query(QueryTab),
    Table(TableTab),
    View(ViewTab),
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
    pub fn new(window: &mut Window, cx: &mut App, variant: TabVariant) -> Self {
        let tab = match &variant {
            TabVariant::Query(number) => TabState::Query(QueryTab::new(window, cx, *number)),
            TabVariant::Table(entity) => TabState::Table(TableTab::new(window, cx, entity.clone())),
            TabVariant::View(entity) => TabState::View(ViewTab::new(window, cx, entity.clone())),
        };

        Self { tab, variant }
    }

    /// Performs any needed behavior for closing the tab.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        match &self.tab {
            TabState::Query(tab) => tab.close(cx),
            TabState::Table(tab) => tab.close(window, cx),
            TabState::View(tab) => tab.close(window, cx),
        }
    }

    /// Returns the content for the tab view.
    pub fn content(&self, window: &mut Window, cx: &App) -> AnyElement {
        match &self.tab {
            TabState::Query(tab) => tab.content(window, cx).into_any_element(),
            TabState::Table(tab) => tab.content(window, cx).into_any_element(),
            TabState::View(tab) => tab.content(window, cx).into_any_element(),
        }
    }

    /// Focuses the content in the tab view.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        match &self.tab {
            TabState::Query(tab) => tab.focus(window, cx),
            TabState::Table(tab) => tab.focus(window, cx),
            TabState::View(tab) => tab.focus(window, cx),
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
    pub fn label(&self) -> SharedString {
        match &self.tab {
            TabState::Query(tab) => tab.label(),
            TabState::Table(tab) => tab.label(),
            TabState::View(tab) => tab.label(),
        }
    }

    /// Returns the variant of the tab view.
    pub fn variant(&self) -> &TabVariant {
        &self.variant
    }
}
