use std::path::PathBuf;

use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window};
use migris::Entity as MigrisEntity;
use uuid::Uuid;

use crate::{
    components::icon::{Icon, IconName},
    shared,
    tabs::{query::QueryTab, table::TableTab, view::ViewTab},
};

pub mod query;
pub mod table;
pub mod view;

/// Initializes configuration for tabs.
pub fn init(cx: &mut App) {
    query::init(cx);
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TabId(Uuid);

impl TabId {
    /// Creates a new [`TabId`].
    fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Returns the underlying UUID.
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

pub enum TabState {
    Query(QueryTab),
    Table(TableTab),
    View(ViewTab),
}

pub enum TabVariant {
    Query(usize, Option<PathBuf>),
    Table(MigrisEntity),
    View(MigrisEntity),
}

pub struct TabView {
    /// The ID for the tab view.
    pub id: TabId,

    /// The state for the tab view.
    pub state: TabState,
}

impl TabView {
    /// Creates a new [`TabView`].
    pub fn new(window: &mut Window, cx: &mut App, variant: TabVariant) -> Self {
        let state = match variant {
            TabVariant::Query(number, path) => TabState::Query(if let Some(path) = path {
                QueryTab::with_path(window, cx, number, path)
            } else {
                QueryTab::new(window, cx, number)
            }),
            TabVariant::Table(entity) => TabState::Table(TableTab::new(window, cx, entity)),
            TabVariant::View(entity) => TabState::View(ViewTab::new(window, cx, entity)),
        };

        Self {
            id: TabId::new(),
            state,
        }
    }

    /// Performs any needed behavior for closing the tab.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        match &self.state {
            TabState::Query(tab) => tab.close(cx),
            TabState::Table(tab) => tab.close(window, cx),
            TabState::View(tab) => tab.close(window, cx),
        }
    }

    /// Returns the content for the tab.
    pub fn content(&self, window: &mut Window, cx: &App) -> AnyElement {
        match &self.state {
            TabState::Query(tab) => tab.content(window, cx).into_any_element(),
            TabState::Table(tab) => tab.content(window, cx).into_any_element(),
            TabState::View(tab) => tab.content(window, cx).into_any_element(),
        }
    }

    /// Focuses the content in the tab.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        match &self.state {
            TabState::Query(tab) => tab.focus(window, cx),
            TabState::Table(tab) => tab.focus(window, cx),
            TabState::View(tab) => tab.focus(window, cx),
        }
    }

    /// Returns the icon for the tab.
    pub fn icon(&self, cx: &App) -> Icon {
        match &self.state {
            TabState::Query(_) => Icon::magenta(cx, IconName::Code),
            TabState::Table(tab) => shared::entity::icon(cx, tab.entity(cx)),
            TabState::View(tab) => shared::entity::icon(cx, tab.entity(cx)),
        }
    }

    /// Returns the label for the tab.
    pub fn label(&self, cx: &App) -> SharedString {
        match &self.state {
            TabState::Query(tab) => tab.label(cx),
            TabState::Table(tab) => tab.label(),
            TabState::View(tab) => tab.label(),
        }
    }

    /// Renames the tab using the given name.
    pub fn rename(&self, cx: &mut App, name: SharedString) {
        match &self.state {
            TabState::Query(tab) => tab.rename(cx, name),
            TabState::Table(_) => {}
            TabState::View(_) => {}
        }
    }

    /// Saves the content of the tab.
    pub fn save(&self, cx: &mut App) {
        match &self.state {
            TabState::Query(tab) => tab.save(cx),
            TabState::Table(_) => {}
            TabState::View(_) => {}
        }
    }

    /// Saves the content of the active tab, prompting the user for a save location.
    pub fn save_as(&self, cx: &mut App) {
        match &self.state {
            TabState::Query(tab) => tab.save_as(cx),
            TabState::Table(_) => {}
            TabState::View(_) => {}
        }
    }

    /// Returns the tooltip for the tab.
    pub fn tooltip(&self, cx: &App) -> Option<SharedString> {
        match &self.state {
            TabState::Query(tab) => tab.tooltip(cx),
            TabState::Table(tab) => Some(tab.tooltip()),
            TabState::View(tab) => Some(tab.tooltip()),
        }
    }
}
