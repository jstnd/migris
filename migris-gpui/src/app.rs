use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription,
    Task, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Root, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    progress::ProgressCircle,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use migris::{Driver, mysql::MySqlConnection};

use crate::{
    assets,
    components::{
        icon::IconName,
        panels::{ConnectionPanel, ConnectionPanelState, TabPanel, TabPanelState},
        settings,
    },
    config::{AppSettings, AppState},
    event::{AppEvent, AppEventKind, EventSource, RunSql},
    models::{ConnectionLoadData, QueryProgress},
    tabs::TabKind,
};

/// Initializes everything the application needs.
///
/// This should always (and only) be called at the application's entry point.
pub fn init(window: &mut Window, cx: &mut App) {
    assets::Themes::init(cx);

    // Set globals for use throughout the application.
    cx.set_global(AppSettings::default());
    cx.set_global(AppState::default());
    AppState::init(window);
}

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
            cx.subscribe_in(&connection_panel, window, |this, _, event, window, cx| {
                this.handle_event(window, cx, event);
            }),
            cx.subscribe_in(&tab_panel, window, |this, _, event, window, cx| {
                this.handle_event(window, cx, event);
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

    fn handle_event(&mut self, window: &mut Window, cx: &mut Context<Self>, event: &AppEvent) {
        match event.kind() {
            AppEventKind::AddConnection => Self::add_connection(cx),
            AppEventKind::OpenEntity(entity) => {
                self.tab_panel.update(cx, |tab_panel, cx| {
                    let tab_kind = TabKind::Table(entity.clone());
                    tab_panel.add_tab(window, cx, tab_kind);
                });
            }
            AppEventKind::RunSql(event_data) => {
                self.run_sql(window, cx, event, event_data.clone());
            }
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

    fn run_sql(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        event: &AppEvent,
        event_data: RunSql,
    ) {
        let id = event.id().cloned();
        let Some(source) = event.source().cloned() else {
            // Return if there's no source as we need
            // somewhere to send the query result afterwards.
            //
            // Ideally, this event kind should always have a source anyways.
            return;
        };

        // TODO: remove this unwrap
        let driver = self.driver.clone().unwrap();

        cx.spawn_in(window, async move |this, cx| {
            let statements = migris::sql::split(&event_data.sql);

            // Initialize the query progress.
            if event_data.show_progress {
                _ = this.update(cx, |this, _| {
                    this.query_progress = Some(QueryProgress::new(statements.len()));
                });
            }

            for (idx, statement) in statements.iter().enumerate() {
                let result = if event_data.stream {
                    driver.query_stream(statement.sql.clone()).await
                } else {
                    driver.query(&statement.sql).await
                };

                match result {
                    Ok(result) => {
                        _ = this.update_in(cx, |this, window, cx| {
                            match source {
                                EventSource::Tab(tab_idx) => {
                                    this.tab_panel.update(cx, |tab_panel, cx| {
                                        tab_panel.load_result(
                                            window,
                                            cx,
                                            tab_idx,
                                            id.clone(),
                                            result,
                                        );
                                    })
                                }
                            }

                            if event_data.show_progress {
                                this.update_query_progress(idx + 1);
                            }

                            cx.notify();
                        });
                    }
                    Err(e) => {
                        println!("QUERY ERROR: {}", e);
                    }
                }
            }

            // Remove the query progress as the statements have finished running.
            if event_data.show_progress {
                _ = this.update(cx, |this, _| {
                    this.query_progress = None;
                });
            }
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

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
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new("settings")
                                    .icon(IconName::Settings)
                                    .ghost()
                                    .xsmall()
                                    .text_color(cx.theme().muted_foreground)
                                    .on_click(|_, window, cx| {
                                        window.open_dialog(cx, |dialog, window, cx| {
                                            settings::settings_dialog(dialog, window, cx)
                                        });
                                    }),
                            )
                            .child("localhost"),
                    )
                    .when_some(self.query_progress.as_ref(), |this, progress| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .child(
                                    ProgressCircle::new("query-progress")
                                        .color(cx.theme().primary)
                                        .value(progress.value()),
                                )
                                .child(progress.label()),
                        )
                    }),
            )
            .children(dialog_layer)
    }
}
