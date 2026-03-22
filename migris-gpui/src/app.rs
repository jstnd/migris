use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, px,
};
use gpui_component::resizable::{h_resizable, resizable_panel};

use crate::components::panels::{ConnectionPanel, ConnectionPanelEvent, ConnectionPanelState};

pub struct Application {
    connection_panel: Entity<ConnectionPanelState>,
    _subscriptions: Vec<Subscription>,
}

impl Application {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let connection_panel = cx.new(|cx| ConnectionPanelState::new(window, cx));
        let _subscriptions = vec![cx.subscribe(
            &connection_panel,
            |_, _, event: &ConnectionPanelEvent, _| match event {
                ConnectionPanelEvent::ConnectionAdded => println!("CONNECTION ADDED"),
                ConnectionPanelEvent::FilterChanged(filter) => {
                    println!("FILTER CHANGED: {}", filter)
                }
            },
        )];

        Self {
            connection_panel,
            _subscriptions,
        }
    }
}

impl Render for Application {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_resizable("application-view")
            .child(
                resizable_panel()
                    .size(px(300.0))
                    .child(ConnectionPanel::new(&self.connection_panel)),
            )
            .child(resizable_panel().child(div().size_full().child("TABS PANEL")))
    }
}
