use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, StatefulInteractiveElement, Styled, Window,
    base::{h_flex, v_flex},
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonVariants},
        tab::{Tab, TabBar},
    },
    div,
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
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for TabPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        v_flex()
            .size_full()
            .child(
                h_flex().id("panel-tab-bar").overflow_x_scroll().child(
                    TabBar::new("panel-tabs")
                        .flex_1()
                        .selected_index(state.active_tab)
                        .track_scroll(&state.scroll_handle)
                        .children(state.tabs.iter().enumerate().map(|(idx, tab)| {
                            let tab = tab.read(cx);

                            Tab::new().child(
                                h_flex()
                                    .id(("panel-tab", idx))
                                    .gap_1p5()
                                    .items_center()
                                    .child(Icon::new(cx, tab.icon()))
                                    .child(tab.label(cx))
                                    .child(
                                        Button::new(("button-close", idx))
                                            .icon(IconName::X)
                                            .ghost()
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground)
                                            .on_click(window.listener_for(
                                                &self.state,
                                                move |state, _, _, cx| {
                                                    state.close_tab(idx);
                                                    cx.stop_propagation();
                                                },
                                            )),
                                    )
                                    .on_hover(window.listener_for(
                                        &self.state,
                                        move |state, is_hovered: &bool, _, cx| {
                                            state.hovered_tab = is_hovered.then_some(idx);
                                            cx.notify();
                                        },
                                    )),
                            )
                        }))
                        .prefix(
                            div().p_1().child(
                                Button::new("button-add-tab")
                                    .icon(IconName::Plus)
                                    .ghost()
                                    .small()
                                    .on_click(window.listener_for(
                                        &self.state,
                                        |state, _, window, cx| {
                                            state.add_query_tab(window, cx);
                                        },
                                    )),
                            ),
                        )
                        .on_click(window.listener_for(&self.state, |state, idx, window, cx| {
                            state.open_tab(window, cx, *idx);
                        })),
                ),
            )
            .when(!state.tabs.is_empty(), |this| {
                this.child(state.active_tab().read(cx).content(window, cx))
            })
    }
}

/// The state used with a [`TabPanel`].
pub struct TabPanelState {
    /// The tabs shown in the panel.
    tabs: Vec<Entity<TabView>>,

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

    /// Adds a new tab to the panel.
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) {
        let tab = cx.new(|cx| TabView::new(window, cx, variant));
        self.tabs.push(tab);

        // Open the newly added tab.
        self.open_tab(window, cx, self.tabs.len() - 1);
    }

    /// Returns the index for the tab displaying the given entity, if one is found.
    pub fn entity_tab(&self, cx: &App, entity: &MigrisEntity) -> Option<usize> {
        self.tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| {
                let tab = tab.read(cx);

                match tab.variant() {
                    TabVariant::Query(_) => false,
                    TabVariant::Table(tab_entity) => tab_entity == entity,
                    TabVariant::View(tab_entity) => tab_entity == entity,
                }
            })
            .map(|(idx, _)| idx)
    }

    /// Opens the tab at the given index.
    pub fn open_tab(&mut self, window: &mut Window, cx: &mut App, idx: usize) {
        self.active_tab = idx;
        self.scroll_handle.scroll_to_item(idx);

        // Focus the opened tab.
        self.active_tab().update(cx, |tab, cx| {
            tab.focus(window, cx);
        });
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> &Entity<TabView> {
        &self.tabs[self.active_tab]
    }

    /// Adds a new query tab to the panel.
    fn add_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let variant = TabVariant::Query(self.next_query_number(cx));
        self.add_tab(window, cx, variant);
    }

    /// Closes the tab at the given index.
    fn close_tab(&mut self, idx: usize) {
        self.tabs.remove(idx);

        // Move the active tab index if the active tab is after the tab that is being closed.
        if self.active_tab >= idx && self.active_tab > 0 {
            self.active_tab -= 1;
        }
    }

    /// Calculates and returns what the next query tab number should be based on the current query tabs.
    ///
    /// The next query tab number should always be equal to the highest current query tab number plus one.
    fn next_query_number(&self, cx: &App) -> usize {
        self.tabs
            .iter()
            .filter_map(|tab| {
                let TabVariant::Query(number) = tab.read(cx).variant() else {
                    return None;
                };

                Some(*number)
            })
            .max()
            .unwrap_or_default()
            + 1
    }
}
