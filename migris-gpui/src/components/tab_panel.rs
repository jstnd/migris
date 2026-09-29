use gpui_kit::{
    App, Context, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    StatefulInteractiveElement, Styled, Window,
    base::{h_flex, v_flex},
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonVariants},
        tab::{Tab, TabBar},
    },
    prelude::FluentBuilder,
};
use migris::Entity as MigrisEntity;

use crate::{
    components::icon::{Icon, IconName},
    tabs::{TabVariant, TabView},
};

#[derive(IntoElement)]
pub struct TabPanel {
    /// The state for the tab panel.
    state: Entity<TabPanelState>,
}

impl TabPanel {
    /// Creates a new [`TabPanel`].
    pub fn new(state: &Entity<TabPanelState>) -> Self {
        Self { state: state.clone() }
    }
}

impl RenderOnce for TabPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        v_flex()
            .size_full()
            .child(
                TabBar::new("panel-tabs")
                    .selected_index(state.active_tab)
                    .track_scroll(&state.scroll_handle)
                    .children(state.tabs.iter().enumerate().map(|(idx, tab)| {
                        Tab::new()
                            .child(
                                h_flex()
                                    .id(("panel-tab", idx))
                                    .gap_1p5()
                                    .items_center()
                                    .child(
                                        Icon::new(cx, tab.icon())
                                            .disabled(state.active_tab != idx && state.hovered_tab != Some(idx)),
                                    )
                                    .child(tab.label())
                                    .when(state.tabs.len() > 1, |this| {
                                        this.child(
                                            Button::new(("btn-close-tab", idx))
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::X)
                                                .text_color(cx.theme().muted_foreground)
                                                .on_click(window.listener_for(
                                                    &self.state,
                                                    move |state, _, window, cx| {
                                                        state.close_tab(window, cx, idx);
                                                        cx.stop_propagation();
                                                    },
                                                )),
                                        )
                                    }),
                            )
                            .on_hover(
                                window.listener_for(&self.state, move |state, is_hovered: &bool, _, cx| {
                                    // Skip handling the unhover event if the currently saved hovered tab does not match this tab.
                                    // This is to prevent issues where the unhover event for a tab fires after the hover event for a different tab.
                                    if state.hovered_tab != Some(idx) && !is_hovered {
                                        return;
                                    }

                                    state.hovered_tab = is_hovered.then_some(idx);
                                    cx.notify();
                                }),
                            )
                    }))
                    .on_click(window.listener_for(&self.state, |state, idx, window, cx| {
                        state.open_tab(window, cx, *idx);
                    })),
            )
            .when(!state.tabs.is_empty(), |this| {
                this.child(state.active_tab().content(window, cx))
            })
    }
}

/// The state used with a [`TabPanel`].
pub struct TabPanelState {
    /// The tabs shown in the panel.
    tabs: Vec<TabView>,

    /// The index of the active tab.
    active_tab: usize,

    /// The index of the currently hovered tab, if any.
    hovered_tab: Option<usize>,

    /// The scroll handle used with the tab bar.
    scroll_handle: ScrollHandle,
}

impl TabPanelState {
    /// Creates a new [`TabPanelState`].
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            hovered_tab: None,
            scroll_handle: ScrollHandle::new(),
        }
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> &TabView {
        &self.tabs[self.active_tab]
    }

    /// Adds a new query tab to the panel.
    pub fn add_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let variant = TabVariant::Query(self.next_query_number());
        self.add_tab(window, cx, variant);
    }

    /// Adds a new tab to the panel.
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) {
        self.tabs.push(TabView::new(window, cx, variant));

        // Open the newly added tab.
        self.open_tab(window, cx, self.tabs.len() - 1);
    }

    /// Closes the active tab.
    pub fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_tab(window, cx, self.active_tab);
    }

    /// Closes the tab at the given index.
    fn close_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, idx: usize) {
        // Do not close if there's only one tab remaining.
        if self.tabs.len() == 1 {
            return;
        }

        self.tabs[idx].close(cx);
        self.tabs.remove(idx);

        // Adjust the active tab if needed.
        let new_tab = if self.active_tab >= idx && self.active_tab > 0 {
            self.active_tab - 1
        } else {
            self.active_tab
        };

        self.open_tab(window, cx, new_tab);
    }

    /// Returns the index for the tab displaying the given entity, if one is found.
    pub fn entity_tab(&self, entity: &MigrisEntity) -> Option<usize> {
        self.tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| match tab.variant() {
                TabVariant::Query(_) => false,
                TabVariant::Table(tab_entity) => tab_entity == entity,
                TabVariant::View(tab_entity) => tab_entity == entity,
            })
            .map(|(idx, _)| idx)
    }

    /// Calculates and returns what the next query tab number should be based on the current query tabs.
    ///
    /// The next query tab number should always be equal to the highest current query tab number plus one.
    fn next_query_number(&self) -> usize {
        self.tabs
            .iter()
            .filter_map(|tab| {
                let TabVariant::Query(number) = tab.variant() else {
                    return None;
                };

                Some(*number)
            })
            .max()
            .unwrap_or_default()
            + 1
    }

    /// Opens the next tab relative to the active tab.
    pub fn open_next_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Do nothing if we're already at the end of the tab bar.
        if self.active_tab == self.tabs.len() - 1 {
            return;
        }

        self.open_tab(window, cx, self.active_tab + 1);
    }

    /// Opens the previous tab relative to the active tab.
    pub fn open_previous_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Do nothing if we're already at the start of the tab bar.
        if self.active_tab == 0 {
            return;
        }

        self.open_tab(window, cx, self.active_tab - 1);
    }

    /// Opens the tab at the given index.
    pub fn open_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, idx: usize) {
        self.active_tab = idx;
        self.scroll_handle.scroll_to_item(idx);

        // Focus the opened tab.
        cx.defer_in(window, |this, window, cx| {
            this.active_tab().focus(window, cx);
        });
    }

    /// Returns a reference to the tabs within the panel.
    pub fn tabs(&self) -> &[TabView] {
        &self.tabs
    }
}
