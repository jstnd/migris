use std::collections::{BTreeMap, HashMap, HashSet};

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    prelude::FluentBuilder, px,
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
use migris::{Entity as MigrisEntity, EntityKind, data::QueryResult};

use crate::{
    components::{connections, icon::IconName},
    event::{AppEvent, AppEventKind, EventId, EventSource},
    tabs::{TabKind, TabView},
};

/// The state for use with the [`ConnectionPanel`].
pub struct ConnectionPanelState {
    search_state: Entity<InputState>,
    tree_state: Entity<TreeState>,

    /// The underlying objects used to build the displayed tree.
    entities: Vec<MigrisEntity>,

    /// A map of entity id's to the respective indexes in the entities list.
    entity_id_map: HashMap<SharedString, usize>,

    /// The id's of the expanded entity tree items; needed to
    /// persist expanded items between actions such as searching.
    expanded: HashSet<SharedString>,

    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<AppEvent> for ConnectionPanelState {}

impl ConnectionPanelState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let _subscriptions =
            vec![
                cx.subscribe(&search_state, |this, _, event: &InputEvent, cx| {
                    if let InputEvent::Change = event {
                        this.load_tree(cx);
                    }
                }),
            ];

        Self {
            search_state,
            tree_state,
            entities: Vec::new(),
            entity_id_map: HashMap::new(),
            expanded: HashSet::new(),
            _subscriptions,
        }
    }

    pub fn load_entities(&mut self, cx: &mut Context<Self>, entities: Vec<MigrisEntity>) {
        self.entities = entities;
        self.build_id_map();
        self.load_tree(cx);
    }

    fn build_id_map(&mut self) {
        self.entity_id_map.clear();

        for (idx, entity) in self.entities.iter().enumerate() {
            self.entity_id_map
                .insert(SharedString::from(entity.id()), idx);
        }
    }

    fn load_tree(&mut self, cx: &mut Context<Self>) {
        self.tree_state.update(cx, |tree_state, cx| {
            let filter = self.search_state.read(cx).value();
            let items = self.entities_to_items(filter);
            tree_state.set_items(items, cx);
        });
    }

    fn entities_to_items(&self, filter: SharedString) -> Vec<TreeItem> {
        let mut items = Vec::new();
        let filter = filter.to_lowercase();
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
                .filter(|entity| filter.is_empty() || entity.name.to_lowercase().contains(&filter))
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

    fn entity(&self, id: &SharedString) -> &MigrisEntity {
        let idx = self.entity_id_map[id];
        &self.entities[idx]
    }

    fn is_expanded(&self, id: &SharedString) -> bool {
        self.expanded.contains(id)
    }
}

#[derive(IntoElement)]
pub struct ConnectionPanel {
    state: Entity<ConnectionPanelState>,
}

impl ConnectionPanel {
    pub fn new(state: &Entity<ConnectionPanelState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for ConnectionPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let search_state = &self.state.read(cx).search_state;

        v_flex()
            .gap_1()
            .p_1()
            .size_full()
            .items_center()
            .child(
                h_flex()
                    .gap_1()
                    .w_full()
                    .child(
                        Input::new(search_state)
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
            .child(tree::tree(
                &self.state.read(cx).tree_state,
                move |idx, entry, _, window, cx| {
                    let entity = self.state.read(cx).entity(&entry.item().id);

                    ListItem::new(idx)
                        .p_0()
                        .text_sm()
                        .child(
                            h_flex()
                                .gap_1()
                                .pl(px(18.0) * entry.depth())
                                .when(entity.kind == EntityKind::Schema, |this| {
                                    this.child(Icon::from(
                                        if self.state.read(cx).is_expanded(&entry.item().id) {
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
                                .child(entry.item().label.clone()),
                        )
                        .on_click(window.listener_for(&self.state, {
                            let entry = entry.clone();
                            move |state, _, _, cx| {
                                let id = entry.item().id.clone();
                                let entity = state.entity(&id);

                                match entity.kind {
                                    EntityKind::Schema => {
                                        if state.is_expanded(&id) {
                                            state.expanded.remove(&id);
                                        } else {
                                            state.expanded.insert(id);
                                        }
                                    }
                                    EntityKind::Table => {
                                        cx.emit(AppEvent::new(AppEventKind::OpenEntity(
                                            entity.clone(),
                                        )));
                                    }
                                    _ => {}
                                }
                            }
                        }))
                },
            ))
    }
}

/// The state for use with the [`TabPanel`].
pub struct TabPanelState {
    /// The tabs shown in the panel.
    tabs: Vec<Entity<TabView>>,

    /// The index of the active tab.
    active_tab: usize,

    /// The index of the currently hovered tab, if any.
    hovered_tab: Option<usize>,

    /// The subscriptions for the panel.
    ///
    /// These will mainly be used for emitting events from tabs upwards to the main application.
    subscriptions: Vec<Subscription>,
}

impl EventEmitter<AppEvent> for TabPanelState {}

impl TabPanelState {
    /// Creates a new [`TabPanelState`].
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            hovered_tab: None,
            subscriptions: Vec::new(),
        }
    }

    /// Adds a new tab to the panel.
    pub fn add_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, kind: TabKind) {
        let tab = cx.new(|cx| TabView::new(window, cx, kind));
        let subscription = cx.subscribe(&tab, |this, _, event, cx| {
            let event = event.clone().with_source(EventSource::Tab(this.active_tab));
            cx.emit(event);
        });

        self.tabs.push(tab);
        self.subscriptions.push(subscription);

        // Set the active tab to be the newly added tab.
        self.active_tab = self.tabs.len() - 1;
    }

    /// Loads the given query result into the tab at the given tab index.
    pub fn load_result(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        tab_idx: usize,
        id: Option<EventId>,
        result: QueryResult,
    ) {
        let tab = &self.tabs[tab_idx];
        tab.update(cx, |tab, cx| {
            tab.load_result(window, cx, id, result);
        });
    }

    /// Returns a reference to the active tab.
    fn active_tab(&self) -> &Entity<TabView> {
        &self.tabs[self.active_tab]
    }

    /// Closes the tab at the given index.
    fn close_tab(&mut self, idx: usize) {
        self.tabs.remove(idx);
        _ = self.subscriptions.remove(idx);

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
                let TabKind::Query(number) = tab.read(cx).kind() else {
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
    state: Entity<TabPanelState>,
}

impl TabPanel {
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
                            .on_click(window.listener_for(&self.state, |state, idx, _, _| {
                                state.active_tab = *idx;
                            }))
                            .children(state.tabs.iter().enumerate().map(|(idx, tab)| {
                                let tab = tab.read(cx);

                                Tab::new().child(
                                    h_flex()
                                        .id(format!("panel-tab-{}", idx))
                                        .gap_1p5()
                                        .items_center()
                                        .on_hover(window.listener_for(
                                            &self.state,
                                            move |state, is_hovered: &bool, _, cx| {
                                                state.hovered_tab = is_hovered.then_some(idx);
                                                cx.notify();
                                            },
                                        ))
                                        .child(Icon::from(tab.icon()).xsmall())
                                        .child(tab.label(cx))
                                        .child(
                                            Button::new(format!("button-close-{}", idx))
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
                                        ),
                                )
                            })),
                    )
                    .child(
                        Button::new("button-add-tab")
                            .icon(IconName::Plus)
                            .ghost()
                            .small()
                            .on_click(window.listener_for(&self.state, |state, _, window, cx| {
                                let tab_kind = TabKind::Query(state.next_query_number(cx));
                                state.add_tab(window, cx, tab_kind);
                            })),
                    ),
            )
            .when(!state.tabs.is_empty(), |this| {
                this.child(state.active_tab().read(cx).content(window, cx))
            })
    }
}
