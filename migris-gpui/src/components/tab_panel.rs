use std::path::PathBuf;

use gpui_kit::{
    Action, App, AppContext, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, StatefulInteractiveElement, Styled, TextRun, Window,
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
use indexmap::IndexMap;
use migris::{Entity as MigrisEntity, EntityKind};

use crate::{
    components::icon::{Icon, IconName},
    tabs::{TabId, TabState, TabVariant, TabView},
};

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum TabPanelAction {
    CloseOtherTabs(TabId),
    CloseTab(TabId),
    RenameTab(TabId),
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
        let selected_index = state.active_tab_index().unwrap_or_default();

        v_flex()
            .size_full()
            .child(
                TabBar::new("panel-tabs")
                    .selected_index(selected_index)
                    .track_scroll(&state.scroll_handle)
                    .children(state.tabs.values().map(|tab| {
                        let tab_id = tab.id;

                        Tab::new()
                            .child(
                                h_flex()
                                    .id(tab_id.uuid())
                                    .gap_1p5()
                                    .items_center()
                                    .child(tab.icon(cx).disabled(
                                        state.active_tab != Some(tab_id) && state.hovered_tab != Some(tab_id),
                                    ))
                                    .child(if state.name_input_tab == Some(tab_id) {
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
                                            Button::new((ElementId::Uuid(tab_id.uuid()), "btn-close-tab"))
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::X)
                                                .text_color(cx.theme().muted_foreground)
                                                .on_click(window.listener_for(
                                                    &self.state,
                                                    move |state, _, window, cx| {
                                                        state.close_tab(window, cx, tab_id);
                                                        cx.stop_propagation();
                                                    },
                                                )),
                                        )
                                    })
                                    .context_menu(move |menu, _, cx| {
                                        menu.menu_with_icon(
                                            "Rename",
                                            Icon::primary(cx, IconName::TextCursorInput),
                                            Box::new(TabPanelAction::RenameTab(tab_id)),
                                        )
                                        .separator()
                                        .menu("Close", Box::new(TabPanelAction::CloseTab(tab_id)))
                                        .menu("Close Others", Box::new(TabPanelAction::CloseOtherTabs(tab_id)))
                                    }),
                            )
                            .when_some(tab.tooltip(cx), |this, tooltip| {
                                this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                            })
                            .on_hover(
                                window.listener_for(&self.state, move |state, is_hovered: &bool, _, cx| {
                                    // Skip handling the unhover event if the currently saved hovered tab does not match this tab.
                                    // This is to prevent issues where the unhover event for a tab fires after the hover event for a different tab.
                                    if state.hovered_tab != Some(tab_id) && !is_hovered {
                                        return;
                                    }

                                    state.hovered_tab = is_hovered.then_some(tab_id);
                                    cx.notify();
                                }),
                            )
                    }))
                    .on_click(window.listener_for(&self.state, |state, idx, window, cx| {
                        state.open_tab_index(window, cx, *idx);
                    })),
            )
            .when_some(state.active_tab(), |this, tab| this.child(tab.content(window, cx)))
            .on_action(window.listener_for(&self.state, |state, action, window, cx| {
                state.handle_action(window, cx, action);
            }))
    }
}

/// The state used with a [`TabPanel`].
pub struct TabPanelState {
    /// The ID of the active tab.
    active_tab: Option<TabId>,

    /// The ID of the hovered tab.
    hovered_tab: Option<TabId>,

    /// The state for the tab name input.
    name_input: Entity<InputState>,

    /// The index of the tab whose name is being edited.
    name_input_tab: Option<TabId>,

    /// The scroll handle used with the tab bar.
    scroll_handle: ScrollHandle,

    /// The tabs shown in the panel.
    tabs: IndexMap<TabId, TabView>,
}

impl TabPanelState {
    /// Creates a new [`TabPanelState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).context_menu(false));
        cx.subscribe(&name_input, |this, _, event: &InputEvent, cx| {
            if let Some(id) = this.name_input_tab
                && let Some(tab) = this.tabs.get(&id)
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
            active_tab: None,
            hovered_tab: None,
            name_input,
            name_input_tab: None,
            scroll_handle: ScrollHandle::new(),
            tabs: IndexMap::new(),
        }
    }

    /// Handles actions originating from the tab panel.
    fn handle_action(&mut self, window: &mut Window, cx: &mut Context<Self>, action: &TabPanelAction) {
        match action {
            TabPanelAction::CloseOtherTabs(_) => {}
            TabPanelAction::CloseTab(id) => self.close_tab(window, cx, *id),
            TabPanelAction::RenameTab(id) => {
                if let Some(tab) = self.tabs.get(id) {
                    let name = tab.label(cx);
                    self.name_input_tab = Some(*id);
                    self.name_input.update(cx, |name_input, cx| {
                        name_input.focus(window, cx);
                        name_input.set_value(name, window, cx);
                    });
                }
            }
        }
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> Option<&TabView> {
        self.active_tab.and_then(|id| self.tabs.get(&id))
    }

    /// Returns the index of the active tab.
    fn active_tab_index(&self) -> Option<usize> {
        self.active_tab.map(|id| self.tab_index(id))
    }

    /// Adds a new query tab to the panel.
    pub fn add_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let variant = TabVariant::Query(self.next_query_number(), None);
        self.add_tab(window, cx, variant);
    }

    /// Adds a new tab to the panel.
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) {
        let tab = TabView::new(window, cx, variant);
        let tab_id = tab.id;
        self.tabs.insert(tab_id, tab);
        self.open_tab(window, cx, tab_id);
    }

    /// Closes the active tab.
    pub fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.active_tab {
            self.close_tab(window, cx, id);
        }
    }

    /// Closes the tab at the given index.
    fn close_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, id: TabId) {
        // Do not close if there's only one tab remaining.
        if self.tabs.len() == 1 {
            return;
        }

        // Remove the tab being closed from the name input.
        if self.name_input_tab == Some(id) {
            self.name_input_tab = None;
        }

        cx.defer_in(window, move |this, window, cx| {
            let Some(tab) = this.tabs.get(&id) else {
                return;
            };

            tab.close(window, cx);

            // Adjust the active tab if needed.
            if this.active_tab == Some(id) {
                let active_tab_idx = this.active_tab_index().unwrap_or_default();
                let new_idx = if active_tab_idx == this.tabs.len() - 1 {
                    active_tab_idx.saturating_sub(1)
                } else {
                    active_tab_idx + 1
                };

                this.open_tab_index(window, cx, new_idx);
            }

            this.tabs.shift_remove(&id);
        });
    }

    /// Returns whether the tab panel is empty.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Calculates and returns what the next query tab number should be based on the current query tabs.
    ///
    /// The next query tab number should always be equal to the highest current query tab number plus one.
    fn next_query_number(&self) -> usize {
        self.tabs
            .values()
            .filter_map(|tab| {
                let TabState::Query(tab) = &tab.state else {
                    return None;
                };

                Some(tab.number())
            })
            .max()
            .unwrap_or_default()
            + 1
    }

    /// Opens a tab for the given entity.
    pub fn open_entity_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) {
        let existing_tab = self.tabs.values().find(|tab| match &tab.state {
            TabState::Query(_) => false,
            TabState::Table(tab) => tab.entity(cx) == &entity,
            TabState::View(tab) => tab.entity(cx) == &entity,
        });

        if let Some(tab) = existing_tab {
            self.open_tab(window, cx, tab.id);
        } else {
            let variant = match entity.kind {
                EntityKind::Table => TabVariant::Table(entity),
                EntityKind::View => TabVariant::View(entity),
                _ => unreachable!(),
            };

            self.add_tab(window, cx, variant);
        }
    }

    /// Opens a query tab with the given path.
    pub fn open_file_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, path: PathBuf) {
        // TODO: check for existing file tab
        let variant = TabVariant::Query(self.next_query_number(), Some(path));
        self.add_tab(window, cx, variant);
    }

    /// Opens the next tab relative to the active tab.
    pub fn open_next_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.active_tab_index() {
            // Do nothing if we're already at the end of the tab bar.
            if idx == self.tabs.len() - 1 {
                return;
            }

            let new_idx = idx + 1;
            self.open_tab_index(window, cx, new_idx);
        }
    }

    /// Opens the previous tab relative to the active tab.
    pub fn open_previous_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.active_tab_index() {
            // Do nothing if we're already at the start of the tab bar.
            if idx == 0 {
                return;
            }

            let new_idx = idx.saturating_sub(1);
            self.open_tab_index(window, cx, new_idx);
        }
    }

    /// Opens the tab with the given ID.
    fn open_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, id: TabId) {
        self.active_tab = Some(id);
        self.scroll_handle.scroll_to_item(self.tab_index(id));

        // Focus the opened tab.
        cx.defer_in(window, |this, window, cx| {
            if let Some(tab) = this.active_tab() {
                tab.focus(window, cx);
            }
        });
    }

    /// Opens the tab at the given index.
    fn open_tab_index(&mut self, window: &mut Window, cx: &mut Context<Self>, idx: usize) {
        if let Some(tab) = self.tab_from_index(idx) {
            self.open_tab(window, cx, tab.id);
        }
    }

    /// Saves the content of the active tab.
    pub fn save(&self, cx: &mut App) {
        if let Some(tab) = self.active_tab() {
            tab.save(cx);
        }
    }

    /// Saves the content of the active tab, prompting the user for a save location.
    pub fn save_as(&self, cx: &mut App) {
        if let Some(tab) = self.active_tab() {
            tab.save_as(cx);
        }
    }

    /// Returns the tab at the given index.
    fn tab_from_index(&self, idx: usize) -> Option<&TabView> {
        self.tabs.get_index(idx).map(|(_, tab)| tab)
    }

    /// Returns the index for the tab with the given ID.
    fn tab_index(&self, id: TabId) -> usize {
        self.tabs.get_index_of(&id).unwrap_or_default()
    }
}
