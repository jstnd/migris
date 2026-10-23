use std::path::PathBuf;

use gpui_kit::{
    Action, App, AppContext, Context, ElementId, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, ScrollHandle, StatefulInteractiveElement, Styled, TextRun, Window,
    base::{
        ResizableState, h_flex,
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
    MoveTabLeft(TabLocation),
    MoveTabRight(TabLocation),
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

        h_resizable("tab-panel")
            .with_state(&state.resizable)
            .children(state.groups.values().map(|group| {
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
                                                    group.active_tab != Some(tab_id)
                                                        && state.hovered_tab != Some(tab_id),
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
                                                    .menu_with_icon(
                                                        "Move Left",
                                                        Icon::primary(cx, IconName::PanelLeftDashed),
                                                        Box::new(TabPanelAction::MoveTabLeft(location)),
                                                    )
                                                    .menu_with_icon(
                                                        "Move Right",
                                                        Icon::primary(cx, IconName::PanelRightDashed),
                                                        Box::new(TabPanelAction::MoveTabRight(location)),
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
                                            this.tooltip(move |window, cx| {
                                                Tooltip::new(tooltip.clone()).build(window, cx)
                                            })
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
    active_tab: Option<TabId>,

    /// The focus handle for the group.
    focus_handle: FocusHandle,

    /// The scroll handle used with the group tab bar.
    scroll_handle: ScrollHandle,

    /// The tabs shown in the group.
    tabs: IndexMap<TabId, TabView>,
}

impl TabGroup {
    /// Creates a new [`TabGroup`].
    fn new(window: &mut Window, cx: &mut Context<TabPanelState>) -> Self {
        let id = TabGroupId::new();
        let focus_handle = cx.focus_handle();
        cx.on_focus_in(&focus_handle, window, move |state, _, _| {
            state.active_group = Some(id);
        })
        .detach();

        Self {
            id,
            active_tab: None,
            focus_handle,
            scroll_handle: ScrollHandle::new(),
            tabs: IndexMap::new(),
        }
    }

    /// Creates a new [`TabGroup`] initialized with the given tab.
    fn with_tab(window: &mut Window, cx: &mut Context<TabPanelState>, tab: TabView) -> Self {
        let mut group = Self::new(window, cx);
        group.active_tab = Some(tab.id);
        group.tabs.insert(tab.id, tab);
        group
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> Option<&TabView> {
        self.active_tab.and_then(|id| self.tabs.get(&id))
    }

    /// Returns the index of the active tab.
    fn active_tab_index(&self) -> usize {
        self.active_tab.map(|id| self.tab_index(id)).unwrap_or_default()
    }

    /// Closes the tab with the given ID.
    fn close_tab(&mut self, window: &mut Window, cx: &mut App, id: TabId) {
        if self.active_tab == Some(id) {
            let active_tab_idx = self.active_tab_index();
            let new_idx = if active_tab_idx == self.tabs.len() - 1 {
                active_tab_idx.saturating_sub(1)
            } else {
                active_tab_idx.saturating_add(1)
            };

            self.open_tab_index(window, cx, new_idx);
        }

        if let Some(tab) = self.tabs.get(&id) {
            tab.close(window, cx);
            self.tabs.shift_remove(&id);
        }
    }

    /// Returns whether the group is empty.
    fn is_empty(&self) -> bool {
        self.tabs.is_empty()
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
        self.active_tab = Some(id);
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

    /// The state for the resizable group panels.
    resizable: Entity<ResizableState>,
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
            resizable: cx.new(|_| ResizableState::default()),
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
            TabPanelAction::MoveTabLeft(location) => {
                let new_group = if self.is_leftmost_group(location.group) {
                    self.add_group(window, cx, Some(0))
                } else {
                    let new_idx = self.group_index(location.group).saturating_sub(1);
                    self.group_from_index(new_idx).unwrap().id
                };

                self.move_tab(window, cx, *location, new_group);
            }
            TabPanelAction::MoveTabRight(location) => {
                let new_group = if self.is_rightmost_group(location.group) {
                    self.add_group(window, cx, None)
                } else {
                    let new_idx = self.group_index(location.group).saturating_add(1);
                    self.group_from_index(new_idx).unwrap().id
                };

                self.move_tab(window, cx, *location, new_group);
            }
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

    /// Returns the index of the active group.
    fn active_group_index(&self) -> usize {
        self.active_group.map(|id| self.group_index(id)).unwrap_or_default()
    }

    /// Returns a mutable reference to the active group.
    fn active_group_mut(&mut self) -> Option<&mut TabGroup> {
        self.active_group.and_then(|group| self.groups.get_mut(&group))
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> Option<&TabView> {
        self.active_group().and_then(|group| group.active_tab())
    }

    /// Adds a new group to the panel.
    fn add_group(&mut self, window: &mut Window, cx: &mut Context<Self>, idx: Option<usize>) -> TabGroupId {
        let group = TabGroup::new(window, cx);
        let group_id = group.id;
        if let Some(idx) = idx {
            self.groups.insert_before(idx, group_id, group);
        } else {
            self.groups.insert(group_id, group);
        }

        // Add new panel with evenly distributed size.
        let groups_len = self.groups.len();
        self.resizable.update(cx, |resizable, cx| {
            let panel_size = resizable.container_size() / groups_len as f32;
            resizable.insert_panel(Some(panel_size), None, cx);
        });

        group_id
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
            let group = TabGroup::with_tab(window, cx, tab);
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
        if let Some(group) = self.active_group()
            && let Some(id) = group.active_tab
        {
            let location = TabLocation {
                group: group.id,
                tab: id,
            };
            self.close_tab(window, cx, location);
        }
    }

    /// Closes the group with the given ID.
    fn close_group(&mut self, window: &mut Window, cx: &mut Context<Self>, id: TabGroupId) {
        if self.active_group == Some(id) {
            let active_group_idx = self.active_group_index();
            let new_idx = if active_group_idx == self.groups.len() - 1 {
                active_group_idx.saturating_sub(1)
            } else {
                active_group_idx.saturating_add(1)
            };

            if let Some(group) = self.group_from_index(new_idx)
                && let Some(id) = group.active_tab
            {
                let location = TabLocation {
                    group: group.id,
                    tab: id,
                };
                self.open_tab(window, cx, location);
            }
        }

        self.groups.shift_remove(&id);
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

            if group.is_empty() {
                this.close_group(window, cx, location.group);
            }
        });
    }

    /// Returns the group at the given index.
    fn group_from_index(&self, idx: usize) -> Option<&TabGroup> {
        self.groups.get_index(idx).map(|(_, group)| group)
    }

    /// Returns the index for the group with the given ID.
    fn group_index(&self, id: TabGroupId) -> usize {
        self.groups.get_index_of(&id).unwrap_or_default()
    }

    /// Returns whether the tab panel is empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Returns whether the group with the given ID is leftmost in the tab panel.
    fn is_leftmost_group(&self, id: TabGroupId) -> bool {
        self.group_index(id) == 0
    }

    /// Returns whether the group with the given ID is rightmost in the tab panel.
    fn is_rightmost_group(&self, id: TabGroupId) -> bool {
        self.group_index(id) == self.groups.len() - 1
    }

    /// Moves the tab at the given location to the given group.
    fn move_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, location: TabLocation, new_group: TabGroupId) {
        let group = &mut self.groups[&location.group];
        let tab = group.tabs.shift_remove(&location.tab).unwrap();

        if group.is_empty() {
            self.close_group(window, cx, location.group);
        } else {
            group.close_tab(window, cx, location.tab);
        }

        let new_location = TabLocation {
            group: new_group,
            tab: location.tab,
        };
        let new_group = &mut self.groups[&new_group];
        new_group.tabs.insert(tab.id, tab);
        self.open_tab(window, cx, new_location);
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
