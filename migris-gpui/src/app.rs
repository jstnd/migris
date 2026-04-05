use std::sync::Arc;

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Task, Window, px,
};
use gpui_component::{
    ActiveTheme, h_flex,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use migris::{Driver, mysql::MySqlConnection};

use crate::{
    components::panels::{ConnectionPanel, ConnectionPanelState, TabPanel, TabPanelState},
    event::{ApplicationEvent, EventSource},
    models::ConnectionLoadData,
};

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
            cx.subscribe(&connection_panel, |_, _, event, cx| {
                if let ApplicationEvent::AddConnection = event {
                    Self::add_connection(cx)
                }
            }),
            cx.subscribe_in(&tab_panel, window, |this, _, event, window, cx| {
                if let ApplicationEvent::RunQuery(query, source) = event {
                    this.run_query(window, cx, query.clone(), *source);
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

    fn run_query(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        query: SharedString,
        source: EventSource,
    ) {
        // TODO: remove this unwrap
        let driver = self.driver.clone().unwrap();

        cx.spawn_in(window, async move |this, cx| {
            for statement in migris::sql::split(&query) {
                let result = driver.query(&statement.sql).await;

                match result {
                    Ok(result) => {
                        let _ = this.update_in(cx, |this, window, cx| match source {
                            EventSource::Tab(idx) => this.tab_panel.update(cx, |state, cx| {
                                state.load_result(window, cx, idx, result);
                                cx.notify();
                            }),
                        });
                    }
                    Err(e) => {
                        println!("QUERY ERROR: {}", e);
                    }
                }
            }
        })
        .detach();
    }
}

impl Render for Application {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_resizable("application-view")
                    .child(
                        resizable_panel()
                            .size(px(300.0))
                            .child(ConnectionPanel::new(&self.connection_panel)),
                    )
                    .child(resizable_panel().child(TabPanel::new(&self.tab_panel))),
            )
            .child(
                h_flex()
                    .px_2()
                    .w_full()
                    .items_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .child("localhost"),
            )
    }
}
