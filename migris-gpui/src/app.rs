use std::{path::Path, sync::Arc};

use gpui_kit::{
    Action, App, AppContext, Context, Entity, InteractiveElement, IntoElement, KeyBinding, ParentElement, Pixels,
    Render, SharedString, Styled, Window,
    base::{h_flex, h_resizable, resizable_panel, v_flex},
    component::{
        ActiveTheme, Sizable, TitleBar,
        button::{Button, ButtonVariants},
        menu::DropdownMenu,
        sidebar::{Sidebar, SidebarItem, SidebarMenuItem},
    },
    img,
    prelude::FluentBuilder,
    px,
};
use migris::{Entity as MigrisEntity, EntityKind, MigrisError, query::Query};

use crate::{
    assets,
    components::{
        self,
        connection_panel::{ConnectionPanel, ConnectionPanelState, ConnectionPanelTab},
        icon::IconName,
        tab_panel::{TabPanel, TabPanelState},
    },
    connections::{ConnectionId, ConnectionManager},
    database::Database,
    events::{EventCallbacks, EventEmitted, EventId, EventManager, EventVariant, LoadEntityEvent, RunSqlEvent},
    history::{QueryHistoryGroup, QueryStatus},
    settings::SettingsManager,
    shared,
    state::AppState,
    tabs::{self, TabVariant},
    types::OpenConnection,
};

/// Initializes everything the application needs.
///
/// This should always (and only) be called at the application's entry point.
pub fn init(window: &mut Window, cx: &mut App, database: Arc<Database>) {
    assets::Themes::init(cx);
    components::init(cx);
    tabs::init(cx);
    init_keybindings(cx);

    // Set globals for use throughout the application.
    cx.set_global(ConnectionManager::new());
    cx.set_global(EventManager::new());

    let app_state = AppState::new(window, cx, database);
    cx.set_global(app_state);

    let settings = SettingsManager::load(cx);
    cx.set_global(settings);

    cx.spawn(async |cx| {
        // TODO: log errors from initializing here
        _ = cx.read_global(|manager: &ConnectionManager, cx| manager.init(cx)).await;
    })
    .detach();
}

/// Initializes application-wide keybinds.
fn init_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-w", ApplicationAction::CloseActiveTab, None),
        KeyBinding::new("ctrl-tab", ApplicationAction::OpenNextTab, None),
        KeyBinding::new("ctrl-shift-tab", ApplicationAction::OpenPreviousTab, None),
        KeyBinding::new("ctrl-t", ApplicationAction::OpenQueryTab, None),
    ]);
}

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum ApplicationAction {
    CloseActiveTab,
    OpenConnectionDialog,
    OpenNextTab,
    OpenPreviousTab,
    OpenQueryTab,
}

pub struct Application {
    /// The currently open connection, if any.
    connection: Option<OpenConnection>,

    /// The state for the connection panel.
    connection_panel: Entity<ConnectionPanelState>,

    /// The state for the tab panel.
    tab_panel: Entity<TabPanelState>,
}

impl Application {
    /// Creates a new [`Application`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            connection: None,
            connection_panel: cx.new(|cx| ConnectionPanelState::new(window, cx)),
            tab_panel: cx.new(|_| TabPanelState::new()),
        }
    }

    fn handle_action(&mut self, window: &mut Window, cx: &mut Context<Self>, action: &ApplicationAction) {
        match action {
            ApplicationAction::CloseActiveTab
            | ApplicationAction::OpenNextTab
            | ApplicationAction::OpenPreviousTab
            | ApplicationAction::OpenQueryTab => {
                self.tab_panel.update(cx, |tab_panel, cx| match action {
                    ApplicationAction::CloseActiveTab => tab_panel.close_active_tab(window, cx),
                    ApplicationAction::OpenNextTab => tab_panel.open_next_tab(window, cx),
                    ApplicationAction::OpenPreviousTab => tab_panel.open_previous_tab(window, cx),
                    ApplicationAction::OpenQueryTab => tab_panel.add_query_tab(window, cx),
                    _ => {}
                });
            }
            ApplicationAction::OpenConnectionDialog => components::open_connection_dialog(window, cx),
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
        let connection = ConnectionManager::global(cx).connection(&connection_id).clone();

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
            let existing_tab = tab_panel.entity_tab(&entity);

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
            let mut continue_execution = true;
            let mut history_group = QueryHistoryGroup::new(connection_id);
            let statements = migris::sql::split(&event.sql);

            // Initialize the query progress.
            if let Some(on_progress) = event.on_progress.clone() {
                _ = cx.update(|window, cx| {
                    on_progress(window, cx, 0, statements.len());
                });
            }

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

                _ = cx.update(|window, cx| match result {
                    Ok(result) => {
                        history.status = QueryStatus::Success;
                        history.duration_ms = result.duration_ms;
                        history.rows_returned = Some(result.data.rows().len() as u64);

                        (event.on_result)(window, cx, result);
                        if let Some(on_progress) = event.on_progress.clone() {
                            on_progress(window, cx, idx + 1, statements.len());
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

            if event.record_history && !history_group.items.is_empty() {
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
}

impl Render for Application {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_panel_tab = self.connection_panel.read(cx).active_tab();

        v_flex()
            .size_full()
            .track_focus(AppState::handle(cx))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_2()
                        .mt_0p5()
                        .child(img(Path::new("./assets/logo-16x16.png")).size_4())
                        .child(render_menu_bar()),
                ),
            )
            .child(
                h_flex()
                    .size_full()
                    .child(
                        Sidebar::new("application-sidebar")
                            .collapsed(true)
                            .children(ConnectionPanelTab::ALL.iter().enumerate().map(|(idx, &tab)| {
                                SidebarMenuItem::new(tab.label())
                                    .text_lg()
                                    .map(|this| if idx == 0 { this.mt_neg_2() } else { this.mt_1() })
                                    .active(active_panel_tab == tab)
                                    .icon(tab.icon())
                                    .on_click(window.listener_for(
                                        &self.connection_panel,
                                        move |connection_panel, _, _, _| {
                                            connection_panel.open_tab(tab);
                                        },
                                    ))
                            }))
                            .footer(
                                SidebarMenuItem::new("Settings")
                                    .collapsed(true)
                                    .text_lg()
                                    .icon(IconName::Settings)
                                    .on_click(|_, window, cx| {
                                        components::open_settings_dialog(window, cx);
                                    })
                                    .render("btn-settings", window, cx),
                            ),
                    )
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
                    ),
            )
            .child(
                h_flex()
                    .px_2()
                    .w_full()
                    .h_6()
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
                            .when_some(self.connection.as_ref(), |this, connection| {
                                this.child(SharedString::from(&connection.connection.name))
                            }),
                    ),
            )
            .on_action(cx.listener(|application, action: &ApplicationAction, window, cx| {
                application.handle_action(window, cx, action);
            }))
            .on_action(cx.listener(|application, action: &EventEmitted, window, cx| {
                application.handle_event(window, cx, &action.0);
            }))
    }
}

fn render_menu_bar() -> impl IntoElement {
    h_flex().gap_1().child(
        Button::new("btn-menu-files")
            .ghost()
            .small()
            .label("File")
            .dropdown_menu(|menu, _, _| menu.menu("Connections", Box::new(ApplicationAction::OpenConnectionDialog))),
    )
}
