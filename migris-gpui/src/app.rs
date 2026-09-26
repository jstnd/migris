use std::sync::Arc;

use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, SharedString, Styled, Window,
    base::{h_flex, h_resizable, resizable_panel, v_flex},
    component::{
        ActiveTheme, Root, Sizable, WindowExt,
        button::{Button, ButtonVariants},
        progress::ProgressCircle,
    },
    prelude::FluentBuilder,
    px,
};
use migris::{Entity as MigrisEntity, EntityKind, MigrisError, query::Query};

use crate::{
    assets,
    components::{
        self,
        connection_panel::{ConnectionPanel, ConnectionPanelState},
        icon::IconName,
        settings,
        tab_panel::{TabPanel, TabPanelState},
    },
    connections::{ConnectionId, ConnectionManager},
    database::Database,
    events::{
        EventCallbacks, EventEmitted, EventId, EventManager, EventVariant, LoadEntityEvent,
        RunSqlEvent,
    },
    history::{QueryHistoryGroup, QueryStatus},
    settings::SettingsManager,
    shared,
    state::AppState,
    tabs::TabVariant,
    types::{OpenConnection, QueryProgress},
};

/// Initializes everything the application needs.
///
/// This should always (and only) be called at the application's entry point.
pub fn init(window: &mut Window, cx: &mut App, database: Arc<Database>) {
    assets::Themes::init(cx);
    components::init(cx);

    // Set globals for use throughout the application.
    cx.set_global(ConnectionManager::new());
    cx.set_global(EventManager::new());

    let app_state = AppState::new(window, cx, database);
    cx.set_global(app_state);

    let settings = SettingsManager::load(cx);
    cx.set_global(settings);

    cx.spawn(async |cx| {
        // TODO: log errors from initializing here
        _ = cx
            .read_global(|manager: &ConnectionManager, cx| manager.init(cx))
            .await;
    })
    .detach();
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
}

impl Application {
    /// Creates a new [`Application`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let connection_panel = cx.new(|cx| ConnectionPanelState::new(window, cx));
        let tab_panel = cx.new(|_| TabPanelState::new());

        Self {
            connection_panel,
            tab_panel,
            connection: None,
            query_progress: None,
        }
    }

    fn handle_event(&mut self, window: &mut Window, cx: &mut Context<Self>, id: &EventId) {
        let Some(event) = EventManager::get(cx, *id) else {
            return;
        };

        match &event.variant {
            EventVariant::LoadEntity(inner) => {
                self.load_entity(window, cx, event.id, inner.clone(), event.callbacks.clone());
            }
            EventVariant::OpenConnection(id) => {
                self.open_connection(window, cx, event.id, *id, event.callbacks.clone())
            }
            EventVariant::OpenEntity(entity) => {
                self.open_entity(window, cx, entity.clone());
                EventManager::complete(cx, id);
            }
            EventVariant::RunSql(inner) => {
                self.run_sql(window, cx, event.id, inner.clone(), event.callbacks.clone());
            }
        }
    }

    fn load_entity(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        event_id: EventId,
        event: LoadEntityEvent,
        callbacks: EventCallbacks,
    ) {
        // TODO: remove this unwrap
        let driver = self.connection.as_ref().unwrap().driver.clone();

        cx.spawn_in(window, async move |_, cx| {
            let result = driver.entity_data(&event.entity).await;
            _ = cx.update(|window, cx| {
                match result {
                    Ok(data) => {
                        (event.on_result)(window, cx, data);
                    }
                    Err(err) => {
                        callbacks.on_error(window, cx, err.to_string());
                    }
                }

                callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event_id);
            })
        })
        .detach();
    }

    fn open_connection(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        event_id: EventId,
        connection_id: ConnectionId,
        callbacks: EventCallbacks,
    ) {
        let connection = ConnectionManager::global(cx)
            .connection(&connection_id)
            .clone();

        cx.spawn_in(window, async move |this, cx| {
            let driver = match shared::create_driver(&connection).await {
                Ok(driver) => driver,
                Err(err) => {
                    _ = cx.update(|window, cx| {
                        callbacks.on_error(window, cx, err.to_string());
                    });
                    return;
                }
            };

            let entities = match driver.entities().await {
                Ok(entities) => entities,
                Err(err) => {
                    _ = cx.update(|window, cx| {
                        callbacks.on_error(window, cx, err.to_string());
                    });
                    return;
                }
            };

            _ = this.update_in(cx, |this, window, cx| {
                this.connection = Some(OpenConnection { connection, driver });
                this.connection_panel.update(cx, |connection_panel, cx| {
                    connection_panel.load_entities(cx, entities);
                });

                // Open a query tab after opening the connection.
                this.tab_panel.update(cx, |tab_panel, cx| {
                    if tab_panel.tabs().is_empty() {
                        tab_panel.add_query_tab(window, cx);
                    }
                });

                callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event_id);
            });
        })
        .detach();
    }

    fn open_entity(&self, window: &mut Window, cx: &mut Context<Self>, entity: MigrisEntity) {
        self.tab_panel.update(cx, |tab_panel, cx| {
            let existing_tab = tab_panel.entity_tab(cx, &entity);

            if let Some(tab_idx) = existing_tab {
                tab_panel.open_tab(window, cx, tab_idx);
            } else {
                let variant = match entity.kind {
                    EntityKind::Table => TabVariant::Table(entity),
                    EntityKind::View => TabVariant::View(entity),
                    _ => unreachable!(),
                };

                tab_panel.add_tab(window, cx, variant);
            }

            cx.notify();
        })
    }

    fn run_sql(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        event_id: EventId,
        event: RunSqlEvent,
        callbacks: EventCallbacks,
    ) {
        // TODO: remove this unwrap
        let connection_id = self.connection.as_ref().unwrap().connection.id;
        let driver = self.connection.as_ref().unwrap().driver.clone();
        let database = AppState::database(cx);

        cx.spawn_in(window, async move |this, cx| {
            let mut history_group = QueryHistoryGroup::new(connection_id);
            let statements = migris::sql::split(&event.sql);

            // Initialize the query progress.
            if event.show_progress {
                _ = this.update(cx, |this, _| {
                    this.query_progress = Some(QueryProgress::new(statements.len()));
                });
            }

            let mut continue_execution = true;
            for (idx, statement) in statements.iter().enumerate() {
                let query = Query::new(&statement.sql, event.token.clone());
                let history = history_group.add(&migris::sql::minify(&query.sql()));

                // We want to continue iterating through the statements even if we stopped execution
                // so the skipped statements can still get recorded within history above.
                if !continue_execution {
                    continue;
                }

                let result = if event.stream {
                    driver.query_stream(&query).await
                } else {
                    driver.query(&query).await
                };

                _ = this.update_in(cx, |this, window, cx| match result {
                    Ok(result) => {
                        history.status = QueryStatus::Success;
                        history.duration_ms = result.duration_ms;
                        history.rows_returned = Some(result.data.rows().len() as u64);

                        (event.on_result)(window, cx, result);
                        if event.show_progress {
                            this.update_query_progress(idx + 1);
                        }
                    }
                    Err(err) => {
                        continue_execution = false;

                        if let MigrisError::QueryCancelled = err {
                            history.status = QueryStatus::Cancelled;
                        } else {
                            history.status = QueryStatus::Failed;
                            history.error = err.to_string();
                            callbacks.on_error(window, cx, err.to_string());
                        }
                    }
                });
            }

            // Remove the query progress as the statements have finished running.
            if event.show_progress {
                _ = this.update(cx, |this, _| {
                    this.query_progress = None;
                });
            }

            if event.record_history {
                // TODO: log errors here
                _ = database.insert_query_history_group(&history_group).await;
                _ = this.update(cx, |this, cx| {
                    this.connection_panel.update(cx, |connection_panel, cx| {
                        connection_panel.add_history(history_group);
                        cx.notify();
                    });
                });
            }

            _ = this.update_in(cx, |_, window, cx| {
                callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event_id);
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

        v_flex()
            .size_full()
            .child(
                h_resizable("application-view")
                    .child(
                        resizable_panel()
                            .size_range(px(250.0)..Pixels::MAX)
                            .size(px(300.0))
                            .child(ConnectionPanel::new(&self.connection_panel)),
                    )
                    .child(resizable_panel().map(|this| {
                        this.child(if self.connection.is_some() {
                            TabPanel::new(&self.tab_panel).into_any_element()
                        } else {
                            components::entry_screen(cx).into_any_element()
                        })
                    })),
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
                                this.child(SharedString::from(&connection.connection.name))
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
            .on_action(
                cx.listener(|application, action: &EventEmitted, window, cx| {
                    application.handle_event(window, cx, &action.0);
                }),
            )
    }
}
