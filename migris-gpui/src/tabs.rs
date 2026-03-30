pub mod query;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, SharedString, Subscription, Window,
};

use crate::{app::ApplicationEvent, components::icon::IconName, tabs::query::QueryTab};

pub enum TabKind {
    Query(usize),
}

enum TabState {
    Query(Entity<QueryTab>),
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

impl EventEmitter<ApplicationEvent> for TabView {}

impl TabView {
    /// Creates a new [`TabView`] of the given kind.
    pub fn new(window: &mut Window, cx: &mut Context<Self>, kind: TabKind) -> Self {
        let tab = match kind {
            TabKind::Query(number) => {
                let tab = cx.new(|cx| QueryTab::new(window, cx, number));
                TabState::Query(tab)
            }
        };

        let _subscription = match &tab {
            TabState::Query(tab) => cx.subscribe(tab, |_, _, event, cx| {
                // Emit the event upwards.
                cx.emit(event.clone());
            }),
        };

        Self {
            kind,
            tab,
            _subscription,
        }
    }

    /// Returns the content for the tab view.
    pub fn content(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        match &self.tab {
            TabState::Query(tab) => tab.read(cx).content(window, cx),
        }
    }

    /// Returns the icon for the tab view.
    pub fn icon(&self) -> IconName {
        match self.kind {
            TabKind::Query(_) => IconName::Code,
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
        }
    }
}
