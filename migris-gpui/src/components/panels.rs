use std::collections::{BTreeMap, HashMap, HashSet};

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    list::ListItem,
    tree::{self, TreeItem, TreeState},
};
use migris::driver::{Entity as MigrisEntity, EntityKind};

use crate::components::icon::IconName;

pub enum ConnectionPanelEvent {
    ConnectionAdded,
}

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
        self.tree_state.update(cx, |state, cx| {
            let filter = self.search_state.read(cx).value();
            let items = self.entities_to_items(filter);
            state.set_items(items, cx);
            cx.notify();
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

impl EventEmitter<ConnectionPanelEvent> for ConnectionPanelState {}

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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let search_state = &self.state.read(cx).search_state;

        div()
            .p_1()
            .v_flex()
            .size_full()
            .gap_1()
            .items_center()
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        Input::new(search_state)
                            .prefix(Icon::from(IconName::Search))
                            .when(!search_state.read(cx).value().is_empty(), |this| {
                                this.suffix(
                                    Button::new("button-search-clear")
                                        .icon(IconName::X)
                                        .compact()
                                        .ghost()
                                        .xsmall()
                                        .tab_stop(false)
                                        .text_color(cx.theme().muted_foreground)
                                        .on_click({
                                            let state = search_state.clone();
                                            move |_, window, cx| {
                                                state.update(cx, |state, cx| {
                                                    state.set_value("", window, cx);
                                                    state.focus(window, cx);
                                                });
                                            }
                                        }),
                                )
                            }),
                    )
                    .child(
                        Button::new("button-add-connection")
                            .icon(IconName::Plus)
                            .tooltip("Add Connection")
                            .compact()
                            .ghost()
                            .on_click(window.listener_for(&self.state, |_, _, _, cx| {
                                cx.emit(ConnectionPanelEvent::ConnectionAdded);
                            })),
                    ),
            )
            .child(tree::tree(
                &self.state.read(cx).tree_state,
                move |idx, entry, _, window, cx| {
                    let entity = self.state.read(cx).entity(&entry.item().id);

                    ListItem::new(idx)
                        .p_0()
                        .child(
                            h_flex()
                                .gap_1()
                                .pl(px(20.0) * entry.depth())
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
                            move |state, _, _, _| {
                                let id = entry.item().id.clone();
                                let entity = state.entity(&id);

                                if entity.kind == EntityKind::Schema {
                                    if state.is_expanded(&id) {
                                        state.expanded.remove(&id);
                                    } else {
                                        state.expanded.insert(id);
                                    }
                                }
                            }
                        }))
                },
            ))
    }
}
