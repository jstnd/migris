use std::collections::BTreeMap;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    Icon, IconName, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    list::ListItem,
    tree::{self, TreeItem, TreeState},
};
use migris::driver::Entity as MigrisEntity;

pub enum ConnectionPanelEvent {
    ConnectionAdded,
    FilterChanged(SharedString),
}

pub struct ConnectionPanelState {
    filter_state: Entity<InputState>,
    tree_state: Entity<TreeState>,

    /// The underlying objects used to build the displayed tree.
    entities: Vec<MigrisEntity>,

    _subscriptions: Vec<Subscription>,
}

impl ConnectionPanelState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_state = cx.new(|cx| InputState::new(window, cx).placeholder("Filter..."));
        let tree_state = cx.new(|cx| TreeState::new(cx));

        let _subscriptions =
            vec![
                cx.subscribe(&filter_state, |_, state, event: &InputEvent, cx| {
                    if let InputEvent::Change = event {
                        let filter = state.read(cx).value();
                        cx.emit(ConnectionPanelEvent::FilterChanged(filter));
                    }
                }),
            ];

        Self {
            filter_state,
            tree_state,
            entities: Vec::new(),
            _subscriptions,
        }
    }

    pub fn load_entities(&mut self, cx: &mut Context<Self>, entities: Vec<MigrisEntity>) {
        self.entities = entities.clone();

        self.tree_state.update(cx, |tree_state, cx| {
            let items = Self::entities_to_items(entities);
            tree_state.set_items(items, cx);
            cx.notify();
        });
    }

    fn entities_to_items(entities: Vec<MigrisEntity>) -> Vec<TreeItem> {
        let mut items = Vec::new();
        let entities_by_schema = entities
            .into_iter()
            .fold(BTreeMap::new(), |mut map, entity| {
                map.entry(entity.schema.clone())
                    .or_insert(Vec::new())
                    .push(entity);
                map
            });

        for (schema, entities) in entities_by_schema {
            let mut children: Vec<TreeItem> = entities
                .into_iter()
                .map(|entity| {
                    TreeItem::new(format!("{}-{}", entity.schema, entity.name), entity.name)
                })
                .collect();

            children.sort_unstable_by(|a, b| a.label.cmp(&b.label));
            let item = TreeItem::new(&schema, &schema).children(children);
            items.push(item);
        }

        items
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
                        Input::new(&self.state.read(cx).filter_state)
                            .cleanable(true)
                            .prefix(Icon::new(IconName::Search)),
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
                |idx, entry, _selected, _window, _cx| {
                    ListItem::new(idx).p_0().child(
                        h_flex()
                            .gap_1()
                            .pl(px(18.0) * entry.depth())
                            .when(entry.is_folder(), |this| {
                                this.child(Icon::new(if entry.is_expanded() {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                }))
                            })
                            .child(entry.item().label.clone()),
                    )
                },
            ))
    }
}
