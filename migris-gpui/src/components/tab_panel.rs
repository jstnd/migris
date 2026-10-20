use std::path::PathBuf;

use gpui_kit::{
    Action, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    StatefulInteractiveElement, Styled, TextRun, Window,
    base::{
        h_flex,
        input::{InputEvent, InputState},
        v_flex,
    },
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonVariants},
        input::Input,
        menu::ContextMenuExt,
        tab::{Tab, TabBar},
        tooltip::Tooltip,
    },
    prelude::FluentBuilder,
    px,
};
use migris::Entity as MigrisEntity;

use crate::{
    components::icon::{Icon, IconName},
    tabs::{TabVariant, TabView},
};

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum TabPanelAction {
    CloseOtherTabs(usize),
    CloseTab(usize),
    RenameTab(usize),
}

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
                                        tab.icon(cx)
                                            .disabled(state.active_tab != idx && state.hovered_tab != Some(idx)),
                                    )
                                    .child(if state.name_input_tab == Some(idx) {
                                        // Calculate the width of the input from the current value.
                                        let value = state.name_input.read(cx).value();
                                        let text_run = TextRun {
                                            len: value.len(),
                                            font: window.text_style().font(),
                                            color: cx.theme().foreground,
                                            background_color: None,
                                            underline: None,
                                            strikethrough: None,
                                        };
                                        let width = window
                                            .text_system()
                                            .shape_line(value, px(14.0), &[text_run], None)
                                            .width()
                                            + px(28.0);

                                        Input::new(&state.name_input).w(width).small().into_any_element()
                                    } else {
                                        tab.label(cx).into_any_element()
                                    })
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
                                    })
                                    .context_menu(move |menu, _, cx| {
                                        menu.menu_with_icon(
                                            "Rename",
                                            Icon::primary(cx, IconName::TextCursorInput),
                                            Box::new(TabPanelAction::RenameTab(idx)),
                                        )
                                        .separator()
                                        .menu("Close", Box::new(TabPanelAction::CloseTab(idx)))
                                        .menu("Close Others", Box::new(TabPanelAction::CloseOtherTabs(idx)))
                                    }),
                            )
                            .when_some(tab.tooltip(cx), |this, tooltip| {
                                this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                            })
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
            .on_action(window.listener_for(&self.state, |state, action, window, cx| {
                state.handle_action(window, cx, action);
            }))
    }
}

/// The state used with a [`TabPanel`].
pub struct TabPanelState {
    /// The index of the active tab.
    active_tab: usize,

    /// The index of the currently hovered tab, if any.
    hovered_tab: Option<usize>,

    /// The state for the tab name input.
    name_input: Entity<InputState>,

    /// The index of the tab whose name is being edited.
    name_input_tab: Option<usize>,

    /// The scroll handle used with the tab bar.
    scroll_handle: ScrollHandle,

    /// The tabs shown in the panel.
    tabs: Vec<TabView>,
}

impl TabPanelState {
    /// Creates a new [`TabPanelState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).context_menu(false));
        cx.subscribe(&name_input, |this, _, event: &InputEvent, cx| {
            if let Some(idx) = this.name_input_tab
                && let Some(tab) = this.tabs.get(idx)
                && matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. })
            {
                this.name_input_tab = None;
                let name = this.name_input.read(cx).value();
                tab.rename(cx, name);
                cx.notify();
            }
        })
        .detach();

        Self {
            active_tab: 0,
            hovered_tab: None,
            name_input,
            name_input_tab: None,
            scroll_handle: ScrollHandle::new(),
            tabs: Vec::new(),
        }
    }

    /// Handles actions originating from the tab panel.
    fn handle_action(&mut self, window: &mut Window, cx: &mut Context<Self>, action: &TabPanelAction) {
        match action {
            TabPanelAction::CloseOtherTabs(_) => {}
            TabPanelAction::CloseTab(idx) => self.close_tab(window, cx, *idx),
            TabPanelAction::RenameTab(idx) => {
                let name = self.tabs[*idx].label(cx);
                self.name_input_tab = Some(*idx);
                self.name_input.update(cx, |name_input, cx| {
                    name_input.focus(window, cx);
                    name_input.set_value(name, window, cx);
                });
            }
        }
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> &TabView {
        &self.tabs[self.active_tab]
    }

    /// Adds a new query tab to the panel.
    pub fn add_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let variant = TabVariant::Query(self.next_query_number(), None);
        self.add_tab(window, cx, variant);
    }

    /// Adds a new tab to the panel.
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) {
        self.tabs.push(TabView::new(window, cx, variant));
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

        // Remove the tab being closed from the name input.
        if self.name_input_tab == Some(idx) {
            self.name_input_tab = None;
        }

        cx.defer_in(window, move |this, window, cx| {
            this.tabs[idx].close(window, cx);
            this.tabs.remove(idx);

            // Adjust the active tab if needed.
            let new_tab = if this.active_tab >= idx && this.active_tab > 0 {
                this.active_tab - 1
            } else {
                this.active_tab
            };

            this.open_tab(window, cx, new_tab);
        });
    }

    /// Returns the index for the tab displaying the given entity, if one is found.
    pub fn entity_tab(&self, entity: &MigrisEntity) -> Option<usize> {
        self.tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| match tab.variant() {
                TabVariant::Query(_, _) => false,
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
                let TabVariant::Query(number, _) = tab.variant() else {
                    return None;
                };

                Some(*number)
            })
            .max()
            .unwrap_or_default()
            + 1
    }

    /// Opens a query tab with the given path.
    pub fn open_file_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, path: PathBuf) {
        // TODO: check for existing file tab
        let variant = TabVariant::Query(self.next_query_number(), Some(path));
        self.add_tab(window, cx, variant);
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

    /// Saves the content of the active tab.
    pub fn save(&self, cx: &mut App) {
        self.active_tab().save(cx);
    }

    /// Saves the content of the active tab, prompting the user for a save location.
    pub fn save_as(&self, cx: &mut App) {
        self.active_tab().save_as(cx);
    }

    /// Returns a reference to the tabs within the panel.
    pub fn tabs(&self) -> &[TabView] {
        &self.tabs
    }
}
