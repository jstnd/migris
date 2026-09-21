use std::collections::{BTreeMap, HashMap, HashSet};

use gpui_kit::{
    Action, App, AppContext, Context, Entity, InteractiveElement, IntoElement, KeyBinding,
    KeystrokeEvent, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window,
    base::{
        TreeItem, TreeState, h_flex,
        input::{InputEvent, InputState},
        v_flex,
    },
    component::{
        input::Input,
        list::ListItem,
        scroll::ScrollableElement,
        tab::{Tab, TabBar},
        tooltip::Tooltip,
        tree,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use migris::{Entity as MigrisEntity, EntityKind};

use crate::{
    components::{
        icon::{Icon, IconName},
        text_ellipsis,
    },
    events::{Event, EventManager, EventVariant},
    history::{QueryHistory, QueryHistoryGroup, QueryStatus},
    shared,
    state::AppState,
};

const CONNECTION_PANEL: &str = "CONNECTION_PANEL";

/// Initializes configuration for the connection panel.
pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        ConnectionPanelAction::OpenSelectedEntity,
        Some(CONNECTION_PANEL),
    )]);
}

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum ConnectionPanelAction {
    OpenSelectedEntity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConnectionPanelTab {
    Connection,
    History,
}

impl ConnectionPanelTab {
    const ALL: [Self; 2] = [Self::Connection, Self::History];

    fn icon(&self) -> IconName {
        match self {
            Self::Connection => IconName::Database,
            Self::History => IconName::RotateCcwClock,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Connection => "Connection",
            Self::History => "History",
        }
    }
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
        let active_tab = self.state.read(cx).active_tab;
        let hovered_tab = self.state.read(cx).hovered_tab;

        v_flex()
            .key_context(CONNECTION_PANEL)
            .size_full()
            .gap_1p5()
            .py_1()
            .child(
                div().w_full().px_1().child(
                    TabBar::new("connection-panel-tabs")
                        .segmented()
                        .selected_index(active_tab as usize)
                        .children(ConnectionPanelTab::ALL.iter().map(|&tab| {
                            Tab::new()
                                .flex_1()
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(Icon::primary(cx, tab.icon()).disabled(
                                            active_tab != tab && hovered_tab != Some(tab),
                                        ))
                                        .child(tab.label()),
                                )
                                .on_click(window.listener_for(
                                    &self.state,
                                    move |state, _, _, _| {
                                        state.active_tab = tab;
                                    },
                                ))
                                .on_hover(window.listener_for(
                                    &self.state,
                                    move |state, is_hovered: &bool, _, _| {
                                        state.hovered_tab = is_hovered.then_some(tab);
                                    },
                                ))
                        })),
                ),
            )
            .child(match active_tab {
                ConnectionPanelTab::Connection => {
                    connection_tab(cx, &self.state).into_any_element()
                }
                ConnectionPanelTab::History => history_tab(cx, &self.state).into_any_element(),
            })
            .on_action(
                window.listener_for(&self.state, |state, action, window, cx| {
                    state.handle_action(window, cx, action);
                }),
            )
    }
}

fn connection_tab(cx: &mut App, state: &Entity<ConnectionPanelState>) -> impl IntoElement {
    v_flex()
        .size_full()
        .gap_1()
        .px_1()
        .child(
            Input::new(&state.read(cx).search_input)
                .cleanable(true)
                .prefix(Icon::new(cx, IconName::Search)),
        )
        .child({
            let state = state.clone();
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
                                this.child(Icon::new(
                                    cx,
                                    if state.read(cx).is_expanded(&entry.item().id) {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    },
                                ))
                            })
                            .child(Icon::new(
                                cx,
                                match entity.kind {
                                    EntityKind::Event => IconName::Calendar,
                                    EntityKind::Function => IconName::SquareFunction,
                                    EntityKind::Procedure => IconName::ScrollText,
                                    EntityKind::Schema => IconName::Database,
                                    EntityKind::Table => IconName::Grid3x3,
                                    EntityKind::Trigger => IconName::Zap,
                                    EntityKind::View => IconName::Eye,
                                },
                            ))
                            .child(text_ellipsis(entry.item().label.clone())),
                    )
                    .on_click(window.listener_for(&state, {
                        let entry = entry.clone();
                        move |state, _, window, cx| {
                            let id = entry.item().id.clone();
                            let entity = state.entity(&id);

                            match entity.kind {
                                EntityKind::Schema => state.toggle_expand(id),
                                EntityKind::Table | EntityKind::View => {
                                    state.open_entity(window, cx, entity);
                                }
                                _ => {}
                            }
                        }
                    }))
            })
        })
}

fn history_tab(cx: &mut App, state: &Entity<ConnectionPanelState>) -> impl IntoElement {
    let Some(history) = &state.read(cx).history else {
        return div().into_any_element();
    };

    v_flex()
        .min_h_0()
        .gap_5()
        .mr_4()
        .pl_2()
        .text_sm()
        .overflow_y_scrollbar()
        .children(history.groups.iter().map(|group| {
            v_flex()
                .gap_0p5()
                .child(div().text_xs().child(group.header()))
                .children(group.items.iter().map(|item| {
                    let item_query = item.query.clone();
                    let tooltip_text = match item.status {
                        QueryStatus::None => "Not Executed".to_string(),
                        QueryStatus::Success => format!(
                            "Success • {} • {}",
                            shared::format_ms(item.duration_ms),
                            item.row_display()
                        ),
                        QueryStatus::Failed => item.error.clone(),
                    };

                    h_flex()
                        .w_full()
                        .gap_0p5()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .id(format!("item-status-{}", item.id))
                                        .child(match item.status {
                                            QueryStatus::None => Icon::new(cx, IconName::Minus),
                                            QueryStatus::Success => {
                                                Icon::green(cx, IconName::Check)
                                            }
                                            QueryStatus::Failed => Icon::red(cx, IconName::X),
                                        })
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(tooltip_text.clone()).build(window, cx)
                                        }),
                                )
                                .child(
                                    div()
                                        .id(format!("item-query-{}", item.id))
                                        .truncate()
                                        .child(SharedString::from(&item.query))
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(migris::sql::format(&item_query))
                                                .build(window, cx)
                                        }),
                                ),
                        )
                }))
        }))
        .into_any_element()
}

/// The state used with a [`ConnectionPanel`].
pub struct ConnectionPanelState {
    /// The active tab within the panel.
    active_tab: ConnectionPanelTab,

    /// The currently hovered tab within the panel.
    hovered_tab: Option<ConnectionPanelTab>,

    /// The state for the search input.
    search_input: Entity<InputState>,

    /// The state for the tree.
    tree: Entity<TreeState>,

    /// The underlying objects used to build the displayed tree.
    entities: Vec<MigrisEntity>,

    /// Tracks the locations of entities within the full list by id.
    entity_map: HashMap<SharedString, usize>,

    /// Tracks the expanded entity tree items.
    ///
    /// This is used to persist expanded items between actions such as searching.
    expanded: HashSet<SharedString>,

    /// The query history to show within the history tab.
    history: Option<QueryHistory>,
}

impl ConnectionPanelState {
    /// Creates a new [`ConnectionPanelState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(shared::SEARCH_PLACEHOLDER));
        let tree = cx.new(|cx| TreeState::new(cx));

        cx.observe_keystrokes(|this, event, _, cx| {
            this.handle_keystroke(cx, event);
        })
        .detach();
        cx.subscribe(&search_input, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                this.load_tree(cx);
            }
        })
        .detach();

        // Load data for history tab.
        let database = AppState::database(cx);
        cx.spawn(async move |this, cx| {
            let items = database.query_history().await.unwrap();
            _ = this.update(cx, |this, _| {
                this.history = Some(QueryHistory::new(items));
            });
        })
        .detach();

        Self {
            active_tab: ConnectionPanelTab::Connection,
            hovered_tab: None,
            search_input,
            tree,
            entities: Vec::new(),
            entity_map: HashMap::new(),
            expanded: HashSet::new(),
            history: None,
        }
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

    /// Adds the given history group to the list within the history tab.
    pub fn add_history(&mut self, group: QueryHistoryGroup) {
        let Some(history) = &mut self.history else {
            return;
        };

        history.groups.insert(0, group);
    }

    /// Loads the given entities into the tree.
    pub fn load_entities(&mut self, cx: &mut Context<Self>, entities: Vec<MigrisEntity>) {
        self.entities = entities;
        self.load_maps();
        self.load_tree(cx);
    }

    fn load_maps(&mut self) {
        self.entity_map.clear();

        for (idx, entity) in self.entities.iter().enumerate() {
            self.entity_map.insert(SharedString::from(entity.id()), idx);
        }
    }

    fn load_tree(&mut self, cx: &mut Context<Self>) {
        let filter = self.search_input.read(cx).value().to_lowercase();
        let filters: Vec<&str> = filter.split('|').filter(|s| !s.is_empty()).collect();
        let items = self.build_tree_items(&filters);
        self.tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
    }

    fn build_tree_items(&self, filters: &[&str]) -> Vec<TreeItem> {
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
                .filter(|entity| {
                    if filters.is_empty() {
                        return true;
                    }

                    let name = entity.name.to_lowercase();
                    filters.iter().any(|f| name.contains(f))
                })
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
}
