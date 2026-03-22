use std::sync::Arc;

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Task,
    Window, div, px,
};
use gpui_component::resizable::{h_resizable, resizable_panel};
use migris::{driver::Driver, mysql::MySqlConnector};

use crate::{
    components::panels::{ConnectionPanel, ConnectionPanelEvent, ConnectionPanelState},
    models::ConnectionLoadData,
};

pub struct Application {
    driver: Option<Arc<dyn Driver>>,
    connection_panel: Entity<ConnectionPanelState>,
    _subscriptions: Vec<Subscription>,
}

impl Application {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let connection_panel = cx.new(|cx| ConnectionPanelState::new(window, cx));
        let _subscriptions = vec![cx.subscribe(
            &connection_panel,
            |_, _, event: &ConnectionPanelEvent, cx| match event {
                ConnectionPanelEvent::ConnectionAdded => Self::connection_added(cx),
            },
        )];

        Self {
            driver: None,
            connection_panel,
            _subscriptions,
        }
    }

    fn connection_added(cx: &mut Context<Self>) {
        let task: Task<Result<ConnectionLoadData, anyhow::Error>> = cx.spawn(async |_, _| {
            let driver: Arc<dyn Driver> =
                Arc::new(MySqlConnector::new_with_pool("mysql://root:root@localhost").await?);

            let entities = driver.entities().await?;
            Ok(ConnectionLoadData { driver, entities })
        });

        cx.spawn(async |this, cx| match task.await {
            Ok(data) => {
                let result = this.update(cx, |this, cx| {
                    this.driver = Some(data.driver);
                    this.connection_panel.update(cx, |state, cx| {
                        state.load_entities(cx, data.entities);
                        cx.notify();
                    });

                    cx.notify();
                });

                if let Err(e) = result {
                    println!("ERROR LOADING: {}", e);
                }
            }
            Err(e) => println!("ERROR LOADING: {}", e),
        })
        .detach();
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
