use std::collections::{BTreeMap, HashMap, HashSet};

use gpui::{
    Action, App, AppContext, Context, Entity, InteractiveElement, IntoElement, KeyBinding,
    KeystrokeEvent, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    list::ListItem,
    tab::{Tab, TabBar},
    tree::{self, TreeItem, TreeState},
    v_flex,
};
use migris::{Entity as MigrisEntity, EntityKind};

use crate::{
    components::{connections, icon::IconName, text_ellipsis},
    events::{Event, EventManager, EventVariant},
    shared,
    tabs::{TabVariant, TabView},
};

const CONNECTION_PANEL: &str = "CONNECTION_PANEL";

/// Initializes configuration for the panel components.
pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        ConnectionPanelAction::OpenSelectedEntity,
        Some(CONNECTION_PANEL),
    )]);
}

/// The state used with a [`ConnectionPanel`].
pub struct ConnectionPanelState {
    /// The state for the search input.
    search_input: Entity<InputState>,

    /// The state for the tree.
    tree: Entity<TreeState>,

    /// The underlying objects used to build the displayed tree.
    entities: Vec<MigrisEntity>,

    /// A map of entity id's to the respective indexes in the entities list.
    entity_map: HashMap<SharedString, usize>,

    /// The id's of the expanded entity tree items; needed to
    /// persist expanded items between actions such as searching.
    expanded: HashSet<SharedString>,

    _subscriptions: Vec<Subscription>,
}

impl ConnectionPanelState {
    /// Creates a new [`ConnectionPanelState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(shared::SEARCH_PLACEHOLDER));
        let tree = cx.new(|cx| TreeState::new(cx));

        let _subscriptions = Vec::from([
            cx.observe_keystrokes(|this, event, _, cx| {
                this.handle_keystroke(cx, event);
            }),
            cx.subscribe(&search_input, |this, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.load_tree(cx);
                }
            }),
        ]);

        Self {
            search_input,
            tree,
            entities: Vec::new(),
            entity_map: HashMap::new(),
            expanded: HashSet::new(),
            _subscriptions,
        }
    }

    /// Loads the given entities into the tree.
    pub fn load_entities(&mut self, cx: &mut Context<Self>, entities: Vec<MigrisEntity>) {
        self.entities = entities;
        self.load_maps();
        self.load_tree(cx);
    }

    /// Handles actions originating from the connection panel.
    fn handle_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: &ConnectionPanelAction,
    ) {
        match action {
            ConnectionPanelAction::OpenSelectedEntity => {
                if let Some(entity) = self.selected_entity(cx)
                    && !entity.is_schema()
                {
                    self.open_entity(window, cx, entity);
                }
            }
        }
    }

    /// Handles keystroke events from inner components.
    fn handle_keystroke(&mut self, cx: &mut Context<Self>, event: &KeystrokeEvent) {
        if let Some(action) = &event.action
            && event
                .context_stack
                .iter()
                .any(|context| context.contains(CONNECTION_PANEL))
        {
            match action.name() {
                "ui::SelectLeft" => {
                    if let Some(schema) = self.selected_schema(cx) {
                        let id = SharedString::from(schema.id());
                        self.expanded.remove(&id);
                    }
                }
                "ui::SelectRight" => {
                    if let Some(schema) = self.selected_schema(cx) {
                        let id = SharedString::from(schema.id());
                        self.expanded.insert(id);
                    }
                }
                _ => {}
            }
        }
    }

    /// Returns the entity with the given id.
    fn entity(&self, id: &SharedString) -> &MigrisEntity {
        let idx = self.entity_map[id];
        &self.entities[idx]
    }

    /// Returns whether the entity with the given id is expanded.
    fn is_expanded(&self, id: &SharedString) -> bool {
        self.expanded.contains(id)
    }

    /// Emits an event to open the given entity.
    fn open_entity(&self, window: &mut Window, cx: &mut Context<Self>, entity: &MigrisEntity) {
        let event = Event::new(EventVariant::OpenEntity(entity.clone()));
        EventManager::emit(window, cx, event);
    }

    /// Returns the selected entity, if any.
    fn selected_entity(&self, cx: &App) -> Option<&MigrisEntity> {
        if let Some(item) = self.tree.read(cx).selected_item() {
            Some(self.entity(&item.id))
        } else {
            None
        }
    }

    /// Returns the selected schema entity, if any.
    fn selected_schema(&self, cx: &App) -> Option<&MigrisEntity> {
        if let Some(entity) = self.selected_entity(cx)
            && entity.is_schema()
        {
            Some(entity)
        } else {
            None
        }
    }

    /// Toggles the expanded state of the entity with the given id.
    fn toggle_expand(&mut self, id: SharedString) {
        if self.is_expanded(&id) {
            self.expanded.remove(&id);
        } else {
            self.expanded.insert(id);
        }
    }

    fn load_maps(&mut self) {
        self.entity_map.clear();

        for (idx, entity) in self.entities.iter().enumerate() {
            self.entity_map.insert(SharedString::from(entity.id()), idx);
        }
    }

    fn load_tree(&mut self, cx: &mut Context<Self>) {
        let filter = self.search_input.read(cx).value().to_lowercase();
        let items = self.build_tree_items(&filter);
        self.tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
    }

    fn build_tree_items(&self, filter: &str) -> Vec<TreeItem> {
        let mut items = Vec::new();
        let entities_by_schema = self
            .entities
            .iter()
            .filter(|entity| entity.kind != EntityKind::Schema)
            .fold(BTreeMap::new(), |mut map, entity| {
                map.entry(entity.schema.clone())
                    .or_insert(Vec::new())
                    .push(entity);
                map
            });

        for (schema, entities) in entities_by_schema {
            let mut children: Vec<TreeItem> = entities
                .into_iter()
                .filter(|entity| filter.is_empty() || entity.name.to_lowercase().contains(filter))
                .map(|entity| TreeItem::new(SharedString::from(entity.id()), &entity.name))
                .collect();

            children.sort_unstable_by(|a, b| a.label.cmp(&b.label));
            let schema_id = SharedString::from(MigrisEntity::schema(&schema).id());
            let item = TreeItem::new(&schema_id, &schema)
                .expanded(self.is_expanded(&schema_id))
                .children(children);

            items.push(item);
        }

        items
    }
}

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum ConnectionPanelAction {
    OpenSelectedEntity,
}

#[derive(IntoElement)]
pub struct ConnectionPanel {
    /// The state for the connection panel.
    state: Entity<ConnectionPanelState>,
}

impl ConnectionPanel {
    /// Creates a new [`ConnectionPanel`].
    pub fn new(state: &Entity<ConnectionPanelState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for ConnectionPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .key_context(CONNECTION_PANEL)
            .gap_1()
            .size_full()
            .items_center()
            .child(
                h_flex()
                    .gap_1()
                    .pt_1()
                    .px_1()
                    .w_full()
                    .child(
                        Input::new(&self.state.read(cx).search_input)
                            .cleanable(true)
                            .prefix(Icon::from(IconName::Search)),
                    )
                    .child(
                        Button::new("button-add-connection")
                            .icon(IconName::Plus)
                            .tooltip("Add Connection")
                            .ghost()
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, window, cx| {
                                    connections::connection_dialog(dialog, window, cx)
                                });
                            }),
                    ),
            )
            .child({
                let state = self.state.clone();
                tree::tree(&state.read(cx).tree, move |idx, entry, _, window, cx| {
                    let entity = state.read(cx).entity(&entry.item().id);

                    ListItem::new(idx)
                        .ml_1()
                        .mr_4()
                        .p_0()
                        .text_sm()
                        .child(
                            h_flex()
                                .gap_1()
                                .px_1()
                                .when(entry.depth() > 0, |this| this.pl(px(22.0) * entry.depth()))
                                .when(entity.is_schema(), |this| {
                                    this.child(Icon::from(
                                        if state.read(cx).is_expanded(&entry.item().id) {
                                            IconName::ChevronDown
                                        } else {
                                            IconName::ChevronRight
                                        },
                                    ))
                                })
                                .child(Icon::from(match entity.kind {
                                    EntityKind::Schema => IconName::Database,
                                    EntityKind::Table => IconName::Grid3x3,
                                    EntityKind::View => IconName::Eye,
                                }))
                                .child(text_ellipsis(entry.item().label.clone())),
                        )
                        .on_click(window.listener_for(&state, {
                            let entry = entry.clone();
                            move |state, _, window, cx| {
                                let id = entry.item().id.clone();
                                let entity = state.entity(&id);

                                match entity.kind {
                                    EntityKind::Schema => state.toggle_expand(id),
                                    EntityKind::Table => {
                                        state.open_entity(window, cx, entity);
                                    }
                                    _ => {}
                                }
                            }
                        }))
                })
            })
            .on_action(
                window.listener_for(&self.state, |state, action, window, cx| {
                    state.handle_action(window, cx, action);
                }),
            )
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
}

impl TabPanelState {
    /// Creates a new [`TabPanelState`].
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            hovered_tab: None,
        }
    }

    /// Adds a new tab to the panel.
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, variant: TabVariant) {
        let tab = cx.new(|cx| TabView::new(window, cx, variant));
        self.tabs.push(tab);

        // Set the active tab to be the newly added tab.
        self.active_tab = self.tabs.len() - 1;
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
                }
            })
            .map(|(idx, _)| idx)
    }

    /// Opens the tab at the given index.
    pub fn open_tab(&mut self, idx: usize) {
        self.active_tab = idx;
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> &Entity<TabView> {
        &self.tabs[self.active_tab]
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
                h_flex()
                    .id("panel-tab-bar")
                    .gap_1()
                    .pr_1()
                    .w_full()
                    .h(px(32.0))
                    .bg(cx.theme().tab_bar)
                    .overflow_x_scroll()
                    .child(
                        TabBar::new("panel-tabs")
                            .selected_index(state.active_tab)
                            .children(state.tabs.iter().enumerate().map(|(idx, tab)| {
                                let tab = tab.read(cx);

                                Tab::new().child(
                                    h_flex()
                                        .id(("panel-tab", idx))
                                        .gap_1p5()
                                        .items_center()
                                        .child(Icon::from(tab.icon()).xsmall())
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
                            .on_click(window.listener_for(&self.state, |state, idx, _, _| {
                                state.open_tab(*idx);
                            })),
                    )
                    .child(
                        Button::new("button-add-tab")
                            .icon(IconName::Plus)
                            .ghost()
                            .small()
                            .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                let variant = TabVariant::Query(state.next_query_number(cx));
                                state.add_tab(window, cx, variant);
                            })),
                    ),
            )
            .when(!state.tabs.is_empty(), |this| {
                this.child(state.active_tab().read(cx).content(window, cx))
            })
    }
}
