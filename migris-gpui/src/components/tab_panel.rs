use std::path::PathBuf;

use gpui_kit::{
    Action, App, AppContext, Context, ElementId, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, ScrollHandle, StatefulInteractiveElement, Styled, TextRun, Window,
    base::{
        h_flex,
        input::{InputEvent, InputState},
        resizable_panel, v_flex,
    },
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonVariants},
        h_resizable,
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
use uuid::Uuid;

use crate::{
    components::icon::{Icon, IconName},
    tabs::{TabId, TabState, TabVariant, TabView},
};

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum TabPanelAction {
    CloseOtherTabs(TabLocation),
    CloseTab(TabLocation),
    RenameTab(TabLocation),
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
        let allow_close_tab = state.allow_close_tab();

        h_resizable("tab-panel").children(state.groups.values().map(|group| {
            resizable_panel().child(
                v_flex()
                    .size_full()
                    .track_focus(&group.focus_handle)
                    .child(
                        TabBar::new(group.id.0)
                            .selected_index(group.active_tab_index())
                            .track_scroll(&group.scroll_handle)
                            .children(group.tabs.values().map(|tab| {
                                let tab_id = tab.id;
                                let location = TabLocation {
                                    group: group.id,
                                    tab: tab_id,
                                };

                                Tab::new()
                                    .child(
                                        h_flex()
                                            .id(tab_id.uuid())
                                            .gap_1p5()
                                            .items_center()
                                            .child(tab.icon(cx).disabled(
                                                group.active_tab != tab_id && state.hovered_tab != Some(tab_id),
                                            ))
                                            .child(if state.name_input_tab == Some(location) {
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
                                            .when(allow_close_tab, |this| {
                                                this.child(
                                                    Button::new((ElementId::Uuid(tab_id.uuid()), "btn-close-tab"))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::X)
                                                        .text_color(cx.theme().muted_foreground)
                                                        .on_click(window.listener_for(
                                                            &self.state,
                                                            move |state, _, window, cx| {
                                                                state.close_tab(window, cx, location);
                                                                cx.stop_propagation();
                                                            },
                                                        )),
                                                )
                                            })
                                            .context_menu(move |menu, _, cx| {
                                                menu.menu_with_icon(
                                                    "Rename",
                                                    Icon::primary(cx, IconName::TextCursorInput),
                                                    Box::new(TabPanelAction::RenameTab(location)),
                                                )
                                                .separator()
                                                .menu("Close", Box::new(TabPanelAction::CloseTab(location)))
                                                .menu(
                                                    "Close Others",
                                                    Box::new(TabPanelAction::CloseOtherTabs(location)),
                                                )
                                            }),
                                    )
                                    .when_some(tab.tooltip(cx), |this, tooltip| {
                                        this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                                    })
                                    .on_click(window.listener_for(&self.state, move |state, _, window, cx| {
                                        state.open_tab(window, cx, location);
                                    }))
                                    .on_hover(window.listener_for(
                                        &self.state,
                                        move |state, is_hovered: &bool, _, cx| {
                                            // Skip handling the unhover event if the currently saved hovered tab does not match this tab.
                                            // This is to prevent issues where the unhover event for a tab fires after the hover event for a different tab.
                                            if state.hovered_tab != Some(tab_id) && !is_hovered {
                                                return;
                                            }

                                            state.hovered_tab = is_hovered.then_some(tab_id);
                                            cx.notify();
                                        },
                                    ))
                            })),
                    )
                    .when_some(group.active_tab(), |this, tab| this.child(tab.content(window, cx)))
                    .on_action(window.listener_for(&self.state, |state, action, window, cx| {
                        state.handle_action(window, cx, action);
                    })),
            )
        }))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct TabGroupId(Uuid);

impl TabGroupId {
    /// Creates a new [`TabGroupId`].
    fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

struct TabGroup {
    /// The ID for the group.
    id: TabGroupId,

    /// The ID of the active tab.
    active_tab: TabId,

    /// The focus handle for the group.
    focus_handle: FocusHandle,

    /// The scroll handle used with the group tab bar.
    scroll_handle: ScrollHandle,

    /// The tabs shown in the group.
    tabs: IndexMap<TabId, TabView>,
}

impl TabGroup {
    /// Creates a new [`TabGroup`].
    fn new(window: &mut Window, cx: &mut Context<TabPanelState>, tab: TabView) -> Self {
        let id = TabGroupId::new();
        let focus_handle = cx.focus_handle();
        cx.on_focus_in(&focus_handle, window, move |state, _, _| {
            state.active_group = Some(id);
        })
        .detach();

        Self {
            id,
            active_tab: tab.id,
            focus_handle,
            scroll_handle: ScrollHandle::new(),
            tabs: IndexMap::from([(tab.id, tab)]),
        }
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> Option<&TabView> {
        self.tabs.get(&self.active_tab)
    }

    /// Returns the index of the active tab.
    fn active_tab_index(&self) -> usize {
        self.tab_index(self.active_tab)
    }

    /// Closes the tab with the given ID.
    fn close_tab(&mut self, window: &mut Window, cx: &mut App, id: TabId) {
        let Some(tab) = self.tabs.get(&id) else {
            return;
        };

        tab.close(window, cx);
        if self.active_tab == id {
            let active_tab_idx = self.active_tab_index();
            let new_idx = if active_tab_idx == self.tabs.len() - 1 {
                active_tab_idx.saturating_sub(1)
            } else {
                active_tab_idx.saturating_add(1)
            };

            self.open_tab_index(window, cx, new_idx);
        }

        self.tabs.shift_remove(&id);
    }

    /// Returns the ID of the next tab relative to the active tab.
    fn next_tab_id(&self) -> Option<TabId> {
        let active_tab_idx = self.active_tab_index();
        if active_tab_idx == self.tabs.len() - 1 {
            return None;
        }

        let next_idx = active_tab_idx.saturating_add(1);
        self.tab_from_index(next_idx).map(|tab| tab.id)
    }

    /// Opens the tab with the given ID.
    fn open_tab(&mut self, window: &mut Window, cx: &mut App, id: TabId) {
        self.active_tab = id;
        self.scroll_handle.scroll_to_item(self.tab_index(id));

        // Focus the opened tab.
        if let Some(tab) = self.active_tab() {
            tab.focus(window, cx);
        }
    }

    /// Opens the tab at the given index.
    fn open_tab_index(&mut self, window: &mut Window, cx: &mut App, idx: usize) {
        if let Some(tab) = self.tab_from_index(idx) {
            self.open_tab(window, cx, tab.id);
        }
    }

    /// Returns the ID of the previous tab relative to the active tab.
    fn previous_tab_id(&self) -> Option<TabId> {
        let active_tab_idx = self.active_tab_index();
        if active_tab_idx == 0 {
            return None;
        }

        let previous_idx = active_tab_idx.saturating_sub(1);
        self.tab_from_index(previous_idx).map(|tab| tab.id)
    }

    /// Returns the tab with the given ID.
    fn tab_from_id(&self, id: TabId) -> Option<&TabView> {
        self.tabs.get(&id)
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

#[derive(Clone, Copy, PartialEq, Eq)]
struct TabLocation {
    /// The ID of the group containing the tab.
    group: TabGroupId,

    /// The ID of the tab.
    tab: TabId,
}

/// The state used with a [`TabPanel`].
pub struct TabPanelState {
    /// The ID of the active group.
    active_group: Option<TabGroupId>,

    /// The groups shown in the panel.
    groups: IndexMap<TabGroupId, TabGroup>,

    /// The ID of the hovered tab.
    hovered_tab: Option<TabId>,

    /// The state for the tab name input.
    name_input: Entity<InputState>,

    /// The tab whose name is being edited.
    name_input_tab: Option<TabLocation>,
}

impl TabPanelState {
    /// Creates a new [`TabPanelState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).context_menu(false));
        cx.subscribe(&name_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                if let Some(location) = this.name_input_tab
                    && let Some(tab) = this.tab_from_location(location)
                {
                    let name = this.name_input.read(cx).value();
                    tab.rename(cx, name);
                }

                this.name_input_tab = None;
                cx.notify();
            }
        })
        .detach();

        Self {
            active_group: None,
            groups: IndexMap::new(),
            hovered_tab: None,
            name_input,
            name_input_tab: None,
        }
    }

    /// Handles actions originating from the tab panel.
    fn handle_action(&mut self, window: &mut Window, cx: &mut Context<Self>, action: &TabPanelAction) {
        match action {
            TabPanelAction::CloseOtherTabs(location) => {
                self.open_tab(window, cx, *location);
                let group = &self.groups[&location.group];
                let removed: Vec<TabId> = group
                    .tabs
                    .keys()
                    .cloned()
                    .filter(|tab_id| *tab_id != location.tab)
                    .collect();
                for id in removed {
                    let location = TabLocation {
                        group: location.group,
                        tab: id,
                    };
                    self.close_tab(window, cx, location);
                }
            }
            TabPanelAction::CloseTab(location) => self.close_tab(window, cx, *location),
            TabPanelAction::RenameTab(location) => {
                let group = &self.groups[&location.group];
                if let Some(tab) = group.tab_from_id(location.tab) {
                    let name = tab.label(cx);
                    self.name_input_tab = Some(*location);
                    self.name_input.update(cx, |name_input, cx| {
                        name_input.focus(window, cx);
                        name_input.set_value(name, window, cx);
                    });
                }
            }
        }
    }

    /// Returns a reference to the active group.
    fn active_group(&self) -> Option<&TabGroup> {
        self.active_group.and_then(|group| self.groups.get(&group))
    }

    /// Returns a mutable reference to the active group.
    fn active_group_mut(&mut self) -> Option<&mut TabGroup> {
        self.active_group.and_then(|group| self.groups.get_mut(&group))
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> Option<&TabView> {
        self.active_group().and_then(|group| group.active_tab())
    }

    /// Adds a new query tab to the active group.
    pub fn add_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_tab(window, cx, TabVariant::Query(self.next_query_number(), None));
    }

    /// Adds a new tab to the active group.
    fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) {
        let tab = TabView::new(window, cx, variant);
        let tab_id = tab.id;
        let location = if self.groups.is_empty() {
            let group = TabGroup::new(window, cx, tab);
            let group_id = group.id;
            self.groups.insert(group_id, group);
            TabLocation {
                group: group_id,
                tab: tab_id,
            }
        } else if let Some(group) = self.active_group_mut() {
            group.tabs.insert(tab.id, tab);
            TabLocation {
                group: group.id,
                tab: tab_id,
            }
        } else {
            return;
        };

        self.open_tab(window, cx, location);
    }

    /// Returns whether to allow closing tabs.
    fn allow_close_tab(&self) -> bool {
        self.groups.len() > 1 || (self.groups.len() == 1 && self.groups[0].tabs.len() > 1)
    }

    /// Closes the active tab.
    pub fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(group) = self.active_group() {
            let location = TabLocation {
                group: group.id,
                tab: group.active_tab,
            };

            self.close_tab(window, cx, location);
        }
    }

    /// Closes the tab at the given location.
    fn close_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, location: TabLocation) {
        if !self.allow_close_tab() {
            return;
        }

        // Remove the tab being closed from the name input.
        if self.name_input_tab == Some(location) {
            self.name_input_tab = None;
        }

        cx.defer_in(window, move |this, window, cx| {
            let group = &mut this.groups[&location.group];
            group.close_tab(window, cx, location.tab);

            if group.tabs.is_empty() {
                this.groups.shift_remove(&location.group);
            }
        });
    }

    /// Returns whether the tab panel is empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Calculates and returns what the next query tab number should be based on the current query tabs.
    ///
    /// The next query tab number should always be equal to the highest current query tab number plus one.
    fn next_query_number(&self) -> usize {
        self.groups
            .values()
            .flat_map(|group| {
                group.tabs.values().filter_map(|tab| {
                    let TabState::Query(tab) = &tab.state else {
                        return None;
                    };

                    Some(tab.number())
                })
            })
            .max()
            .unwrap_or_default()
            + 1
    }

    /// Opens a tab for the given entity.
    pub fn open_entity_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) {
        let location = self.groups.values().find_map(|group| {
            group
                .tabs
                .values()
                .find(|tab| match &tab.state {
                    TabState::Query(_) => false,
                    TabState::Table(tab) => tab.entity(cx) == &entity,
                    TabState::View(tab) => tab.entity(cx) == &entity,
                })
                .map(|tab| TabLocation {
                    group: group.id,
                    tab: tab.id,
                })
        });

        if let Some(location) = location {
            self.open_tab(window, cx, location);
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
        self.add_tab(window, cx, TabVariant::Query(self.next_query_number(), Some(path)));
    }

    /// Opens the next tab relative to the active tab.
    pub fn open_next_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(group) = self.active_group()
            && let Some(id) = group.next_tab_id()
        {
            let location = TabLocation {
                group: group.id,
                tab: id,
            };

            self.open_tab(window, cx, location);
        }
    }

    /// Opens the previous tab relative to the active tab.
    pub fn open_previous_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(group) = self.active_group()
            && let Some(id) = group.previous_tab_id()
        {
            let location = TabLocation {
                group: group.id,
                tab: id,
            };

            self.open_tab(window, cx, location);
        }
    }

    /// Opens the tab at the given location.
    fn open_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, location: TabLocation) {
        cx.defer_in(window, move |this, window, cx| {
            let group = &mut this.groups[&location.group];
            group.open_tab(window, cx, location.tab);
        });
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

    /// Returns the tab at the given location.
    fn tab_from_location(&self, location: TabLocation) -> Option<&TabView> {
        self.groups[&location.group].tab_from_id(location.tab)
    }
}
