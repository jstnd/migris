use std::sync::Arc;

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Task, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, h_flex,
    progress::ProgressCircle,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use migris::{Driver, mysql::MySqlConnection};

use crate::{
    components::panels::{ConnectionPanel, ConnectionPanelState, TabPanel, TabPanelState},
    event::{ApplicationEvent, EventSource},
    models::{ConnectionLoadData, QueryProgress},
};

pub struct Application {
    driver: Option<Arc<dyn Driver>>,
    connection_panel: Entity<ConnectionPanelState>,
    tab_panel: Entity<TabPanelState>,

    /// Tracks the progress of the running query, if any.
    query_progress: Option<QueryProgress>,

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
            query_progress: None,
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
                _ = this.update(cx, |this, cx| {
                    this.driver = Some(data.driver);
                    this.connection_panel.update(cx, |connection_panel, cx| {
                        connection_panel.load_entities(cx, data.entities);
                    });

                    cx.notify();
                });
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
            let statements = migris::sql::split(&query);

            // Initialize the query progress.
            _ = this.update(cx, |this, _| {
                this.query_progress = Some(QueryProgress::new(statements.len()));
            });

            for (idx, statement) in statements.iter().enumerate() {
                let result = driver.query(&statement.sql).await;

                match result {
                    Ok(result) => {
                        _ = this.update_in(cx, |this, window, cx| {
                            match source {
                                EventSource::Tab(idx) => {
                                    this.tab_panel.update(cx, |tab_panel, cx| {
                                        tab_panel.load_result(window, cx, idx, result);
                                    })
                                }
                            }

                            this.update_query_progress(idx + 1);
                            cx.notify();
                        });
                    }
                    Err(e) => {
                        println!("QUERY ERROR: {}", e);
                    }
                }
            }

            // Remove the query progress as the statements have finished running.
            _ = this.update(cx, |this, _| {
                this.query_progress = None;
            });
        })
        .detach();
    }

    fn update_query_progress(&mut self, complete: usize) {
        if let Some(progress) = &mut self.query_progress {
            progress.update(complete);
        }
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
                    .justify_between()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .child("localhost")
                    .when_some(self.query_progress.as_ref(), |this, progress| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    ProgressCircle::new("query-progress")
                                        .color(cx.theme().primary)
                                        .value(progress.value()),
                                )
                                .child(progress.label()),
                        )
                    }),
            )
    }
}
