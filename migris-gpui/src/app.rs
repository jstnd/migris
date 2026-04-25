use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Root, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    progress::ProgressCircle,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};

use crate::{
    assets,
    components::{
        icon::IconName,
        panels::{ConnectionPanel, ConnectionPanelState, TabPanel, TabPanelState},
        settings,
    },
    connections::{ConnectionId, ConnectionManager},
    event::{AppEvent, AppEventKind, EventSource, RunSql},
    settings::AppSettings,
    state::AppState,
    tabs::TabKind,
    types::{OpenConnection, QueryProgress},
};

/// Initializes everything the application needs.
///
/// This should always (and only) be called at the application's entry point.
pub fn init(window: &mut Window, cx: &mut App) {
    assets::Themes::init(cx);

    // Set globals for use throughout the application.
    cx.set_global(AppSettings::default());
    cx.set_global(ConnectionManager::load());

    let app_state = AppState::new(window, cx);
    cx.set_global(app_state);
}

pub struct Application {
    /// The state for the connection panel.
    connection_panel: Entity<ConnectionPanelState>,

    /// The state for the tab panel.
    tab_panel: Entity<TabPanelState>,

    /// The currently open connection, if any.
    connection: Option<OpenConnection>,

    /// The progress of the running query, if any.
    query_progress: Option<QueryProgress>,

    /// The subscriptions for the application.
    ///
    /// These will pick up events from various locations and perform the needed work; see [`Self::handle_event`].
    _subscriptions: Vec<Subscription>,
}

impl Application {
    /// Creates a new [`Application`].
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
            cx.subscribe_in(
                &AppState::global(cx).connection_dialog_state.clone(),
                window,
                |this, _, event, window, cx| {
                    this.handle_event(window, cx, event);
                },
            ),
        ]);

        Self {
            connection_panel,
            tab_panel,
            connection: None,
            query_progress: None,
            _subscriptions,
        }
    }

    fn handle_event(&mut self, window: &mut Window, cx: &mut Context<Self>, event: &AppEvent) {
        match event.kind() {
            AppEventKind::OpenConnection(id) => self.open_connection(window, cx, *id),
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

    fn open_connection(&self, window: &mut Window, cx: &mut Context<Self>, id: ConnectionId) {
        let connection = ConnectionManager::global(cx).connection(&id).clone();

        cx.spawn_in(window, async |this, cx| {
            let Ok(driver) = migris::driver(connection.options()).await else {
                println!("DRIVER FAILED");
                return;
            };

            let Ok(entities) = driver.entities().await else {
                println!("ENTITIES FAILED");
                return;
            };

            _ = this.update_in(cx, |this, window, cx| {
                this.connection = Some(OpenConnection { connection, driver });
                this.connection_panel.update(cx, |connection_panel, cx| {
                    connection_panel.load_entities(cx, entities);
                });

                // Close the connection dialog here after the connection was successfully loaded.
                window.close_dialog(cx);
            });
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
        let driver = self.connection.as_ref().unwrap().driver.clone();

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
                            .when_some(self.connection.as_ref(), |this, connection| {
                                this.child(connection.connection.name())
                            }),
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
