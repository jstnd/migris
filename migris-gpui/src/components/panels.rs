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

pub enum ConnectionPanelEvent {
    ConnectionAdded,
    FilterChanged(SharedString),
}

pub struct ConnectionPanelState {
    filter_state: Entity<InputState>,
    tree_state: Entity<TreeState>,
    _subscriptions: Vec<Subscription>,
}

impl ConnectionPanelState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_state = cx.new(|cx| InputState::new(window, cx).placeholder("Filter..."));
        let tree_state = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("src", "src")
                    .expanded(true)
                    .child(TreeItem::new("src/lib.rs", "lib.rs"))
                    .child(TreeItem::new("src/main.rs", "main.rs")),
                TreeItem::new("Cargo.toml", "Cargo.toml"),
                TreeItem::new("README.md", "README.md"),
            ])
        });

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
            _subscriptions,
        }
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
