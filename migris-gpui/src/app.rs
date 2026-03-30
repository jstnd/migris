use std::sync::Arc;

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Subscription,
    Task, Window, px,
};
use gpui_component::resizable::{h_resizable, resizable_panel};
use migris::{Driver, mysql::MySqlConnection};

use crate::{
    components::panels::{ConnectionPanel, ConnectionPanelState, TabPanel, TabPanelState},
    models::ConnectionLoadData,
};

#[derive(Clone)]
pub enum ApplicationEvent {
    AddConnection,
    RunQuery(SharedString),
}

pub struct Application {
    driver: Option<Arc<dyn Driver>>,
    connection_panel: Entity<ConnectionPanelState>,
    tab_panel: Entity<TabPanelState>,
    _subscriptions: Vec<Subscription>,
}

impl Application {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let connection_panel = cx.new(|cx| ConnectionPanelState::new(window, cx));
        let tab_panel = cx.new(|_| TabPanelState::new());
        let _subscriptions = Vec::from([
            cx.subscribe(&connection_panel, |_, _, event: &ApplicationEvent, cx| {
                if let ApplicationEvent::AddConnection = event {
                    Self::add_connection(cx)
                }
            }),
            cx.subscribe(&tab_panel, |_, _, event: &ApplicationEvent, _| {
                if let ApplicationEvent::RunQuery(query) = event {
                    Self::run_query(query.clone());
                }
            }),
        ]);

        Self {
            driver: None,
            connection_panel,
            tab_panel,
            _subscriptions,
        }
    }

    fn add_connection(cx: &mut Context<Self>) {
        let task: Task<Result<ConnectionLoadData, anyhow::Error>> = cx.spawn(async |_, _| {
            let driver: Arc<dyn Driver> =
                Arc::new(MySqlConnection::new("mysql://root:root@localhost").await?);

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

    fn run_query(query: SharedString) {
        println!("{}", query);
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
            .child(resizable_panel().child(TabPanel::new(&self.tab_panel)))
    }
}
