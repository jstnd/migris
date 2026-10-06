use std::{path::Path, sync::Arc};

use gpui_kit::{
    Action, App, AppContext, Context, Entity, FocusHandle, Hsla, InteractiveElement, IntoElement, KeyBinding,
    MouseButton, ParentElement, Pixels, Render, SharedString, Styled, Window,
    base::{
        ColorPickerEvent, ColorPickerState, ResizablePanelEvent, ResizableState, h_flex, h_resizable, resizable_panel,
        v_flex,
    },
    component::{
        ActiveTheme, Colorize, Sizable, TitleBar,
        button::{Button, ButtonVariants},
        color_picker::ColorPicker,
        menu::DropdownMenu,
        sidebar::{Sidebar, SidebarItem, SidebarMenuItem},
    },
    div, img,
    prelude::FluentBuilder,
    px,
};
use migris::{Entity as MigrisEntity, EntityKind, MigrisError, query::Query};

use crate::{
    assets,
    components::{
        self,
        icon::IconName,
        side_panel::{SidePanel, SidePanelState, SidePanelTab},
        tab_panel::{TabPanel, TabPanelState},
    },
    connections::{ConnectionId, ConnectionManager},
    database::Database,
    events::{Event, EventEmitted, EventId, EventManager, EventVariant, LoadEntityEvent, RunSqlEvent},
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
        KeyBinding::new("secondary-w", ApplicationAction::CloseActiveTab, None),
        KeyBinding::new("secondary-tab", ApplicationAction::OpenNextTab, None),
        KeyBinding::new("secondary-shift-tab", ApplicationAction::OpenPreviousTab, None),
        KeyBinding::new("secondary-t", ApplicationAction::OpenQueryTab, None),
        KeyBinding::new("secondary-,", ApplicationAction::OpenSettings, None),
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
    OpenSettings,
}

const DEFAULT_SIDE_PANEL_WIDTH: Pixels = px(300.0);

pub struct Application {
    /// The state for the color picker.
    ///
    /// This is used for easily changing the connection's associated color.
    color_picker: Entity<ColorPickerState>,

    /// The focus handle for the color picker.
    color_picker_focus_handle: FocusHandle,

    /// The state for the resizable panels.
    resizable: Entity<ResizableState>,

    /// The state for the side panel.
    side_panel: Entity<SidePanelState>,

    /// Whether the side panel is expanded.
    side_panel_expanded: bool,

    /// The width of the side panel.
    side_panel_width: Pixels,

    /// The state for the tab panel.
    tab_panel: Entity<TabPanelState>,
}

impl Application {
    /// Creates a new [`Application`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));
        let color_picker_focus_handle = cx.focus_handle();
        cx.subscribe(&color_picker, |_, _, event, cx| match event {
            ColorPickerEvent::Change(color) => {
                // This event fires after every single color change (including when changing color via sliders),
                // so we want to only update the color in memory for the connection.
                let color = color.as_ref().map(|hsla| hsla.to_hex());
                let connection_id = AppState::connection_unchecked(cx).id();
                ConnectionManager::global_mut(cx).connection_mut(&connection_id).color = color;
            }
        })
        .detach();
        cx.on_focus_out(&color_picker_focus_handle, window, |this, _, _, cx| {
            // Persist the selected color within the application's database only after the user finishes picking the color.
            let color = this.color_picker.read(cx).value().map(|hsla| hsla.to_hex());
            let connection_id = AppState::connection_unchecked(cx).id();
            cx.spawn(async move |_, cx| {
                _ = cx
                    .read_global(|manager: &ConnectionManager, cx| {
                        manager.update_connection_color(cx, connection_id, color)
                    })
                    .await;
            })
            .detach();
        })
        .detach();

        let resizable = cx.new(|_| ResizableState::default());
        cx.subscribe(&resizable, |this, resizable, event, cx| {
            match event {
                ResizablePanelEvent::Resized => {
                    // Track any size changes to the side panel while it's visible.
                    if this.side_panel_expanded {
                        this.side_panel_width = resizable.read(cx).sizes()[0];
                    }
                }
            }
        })
        .detach();

        Self {
            color_picker,
            color_picker_focus_handle,
            resizable,
            side_panel: cx.new(|cx| SidePanelState::new(window, cx)),
            side_panel_expanded: true,
            side_panel_width: DEFAULT_SIDE_PANEL_WIDTH,
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
            ApplicationAction::OpenSettings => components::open_settings_dialog(window, cx),
        }
    }

    fn handle_event(&mut self, window: &mut Window, cx: &mut Context<Self>, id: &EventId) {
        let Some(event) = EventManager::get(cx, *id) else {
            return;
        };

        let event = event.clone();
        let event_variant = event.variant.clone();
        match event_variant {
            EventVariant::KillProcess(process_id) => self.kill_process(window, cx, event, process_id),
            EventVariant::LoadEntity(inner) => {
                self.load_entity(window, cx, event, inner);
            }
            EventVariant::OpenConnection(id) => self.open_connection(window, cx, event, id),
            EventVariant::OpenEntity(entity) => {
                self.open_entity(window, cx, entity);
                EventManager::complete(cx, id);
            }
            EventVariant::RunSql(inner) => {
                self.run_sql(window, cx, event, inner);
            }
        }
    }

    fn kill_process(&self, window: &mut Window, cx: &mut Context<Self>, event: Event, process_id: u64) {
        let driver = AppState::connection_unchecked(cx).driver();
        cx.spawn_in(window, async move |_, cx| {
            let result = driver.kill_process(process_id).await;
            _ = cx.update(|window, cx| {
                if let Err(err) = result {
                    println!("{}", err);
                    event.callbacks.on_error(window, cx, err.to_string());
                }

                event.callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event.id);
            });
        })
        .detach();
    }

    fn load_entity(&self, window: &mut Window, cx: &mut Context<Self>, event: Event, load_event: LoadEntityEvent) {
        let driver = AppState::connection_unchecked(cx).driver();
        cx.spawn_in(window, async move |_, cx| {
            let result = driver.entity_data(&load_event.entity).await;
            _ = cx.update(|window, cx| {
                match result {
                    Ok(data) => {
                        (load_event.on_result)(window, cx, data);
                    }
                    Err(err) => {
                        event.callbacks.on_error(window, cx, err.to_string());
                    }
                }

                event.callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event.id);
            })
        })
        .detach();
    }

    fn open_connection(&self, window: &mut Window, cx: &mut Context<Self>, event: Event, connection_id: ConnectionId) {
        let connection = ConnectionManager::global(cx).connection(&connection_id).clone();
        cx.spawn_in(window, async move |this, cx| {
            let driver = match shared::create_driver(&connection).await {
                Ok(driver) => driver,
                Err(err) => {
                    _ = cx.update(|window, cx| {
                        event.callbacks.on_error(window, cx, err.to_string());
                    });
                    return;
                }
            };

            let entities = match driver.entities().await {
                Ok(entities) => entities,
                Err(err) => {
                    _ = cx.update(|window, cx| {
                        event.callbacks.on_error(window, cx, err.to_string());
                    });
                    return;
                }
            };

            // Update the last connected date of the connection.
            _ = cx
                .read_global(|manager: &ConnectionManager, _, cx| {
                    manager.update_connection_last_connected(cx, connection_id)
                })
                .unwrap()
                .await;

            _ = this.update_in(cx, |this, window, cx| {
                let open_connection = OpenConnection::new(connection_id, driver, entities);
                AppState::set_connection(cx, open_connection);

                // Load the connection's color into the color picker.
                let connection = ConnectionManager::global(cx).connection(&connection_id);
                if let Some(color) = &connection.color
                    && let Ok(hsla) = Hsla::parse_hex(color)
                {
                    this.color_picker.update(cx, |color_picker, cx| {
                        color_picker.set_value(hsla, window, cx);
                    });
                }

                // Load the connection's entities into the side panel.
                this.side_panel.update(cx, |side_panel, cx| {
                    side_panel.load_entities(cx);
                });

                // Open a query tab after opening the connection.
                this.tab_panel.update(cx, |tab_panel, cx| {
                    if tab_panel.tabs().is_empty() {
                        tab_panel.add_query_tab(window, cx);
                    }
                });

                event.callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event.id);
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

    fn run_sql(&self, window: &mut Window, cx: &mut Context<Self>, event: Event, run_event: RunSqlEvent) {
        let connection = AppState::connection_unchecked(cx);
        let connection_id = connection.id();
        let driver = connection.driver();
        let database = AppState::database(cx);

        cx.spawn_in(window, async move |this, cx| {
            let mut continue_execution = true;
            let mut history_group = QueryHistoryGroup::new(connection_id);
            let statements = migris::sql::split(&run_event.sql);

            // Initialize the query progress.
            if let Some(on_progress) = run_event.on_progress.clone() {
                _ = cx.update(|window, cx| {
                    on_progress(window, cx, 0, statements.len());
                });
            }

            for (idx, statement) in statements.iter().enumerate() {
                let query = Query::new(&statement.sql, run_event.token.clone());
                let history = history_group.add(&migris::sql::minify(&query.sql()));

                // We want to continue iterating through the statements even if we stopped execution
                // so the skipped statements can still get recorded within history above.
                if !continue_execution {
                    continue;
                }

                let result = if run_event.stream {
                    driver.query_stream(&query).await
                } else {
                    driver.query(&query).await
                };

                _ = cx.update(|window, cx| match result {
                    Ok(result) => {
                        history.status = QueryStatus::Success;
                        history.duration_ms = result.duration_ms;
                        history.rows_returned = Some(result.data.rows().len() as u64);

                        (run_event.on_result)(window, cx, result);
                        if let Some(on_progress) = run_event.on_progress.clone() {
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
                            event.callbacks.on_error(window, cx, err.to_string());
                        }
                    }
                });
            }

            if run_event.record_history && !history_group.items.is_empty() {
                // TODO: log errors here
                _ = database.insert_query_history_group(&history_group).await;
                _ = this.update(cx, |this, cx| {
                    this.side_panel.update(cx, |side_panel, cx| {
                        side_panel.add_history(history_group);
                        cx.notify();
                    });
                });
            }

            _ = this.update_in(cx, |_, window, cx| {
                event.callbacks.on_complete(window, cx);
                EventManager::complete(cx, &event.id);
            });
        })
        .detach();
    }

    /// Toggles the visibility of the side panel.
    fn toggle_side_panel(&mut self, cx: &mut Context<Self>) {
        self.side_panel_expanded = !self.side_panel_expanded;
        if self.side_panel_expanded {
            self.resizable.update(cx, |resizable, cx| {
                // Add back the panel with its previous width.
                resizable.insert_panel(Some(self.side_panel_width), Some(0), cx);
            });
        }

        cx.notify();
    }
}

impl Render for Application {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_panel_tab = self.side_panel.read(cx).active_tab();

        v_flex()
            .size_full()
            .track_focus(AppState::handle(cx))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .mt_0p5()
                        .mx_1()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_2()
                                .child(img(Path::new("./assets/logo-16x16.png")).size_4())
                                .child(render_menu_bar()),
                        )
                        .child(
                            h_flex()
                                .child(
                                    Button::new("btn-toggle-side-panel")
                                        .ghost()
                                        .small()
                                        .icon(if self.side_panel_expanded {
                                            IconName::PanelLeftClose
                                        } else {
                                            IconName::PanelLeftOpen
                                        })
                                        .tooltip("Toggle Side Panel")
                                        .on_click(cx.listener(|application, _, _, cx| {
                                            application.toggle_side_panel(cx);
                                        })),
                                )
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    // This is needed to stop drag events from the title bar taking priority over button clicks.
                                    cx.stop_propagation();
                                }),
                        ),
                ),
            )
            .child(
                h_flex()
                    .size_full()
                    .child(
                        Sidebar::new("application-sidebar")
                            .collapsed(true)
                            .children(SidePanelTab::ALL.iter().enumerate().map(|(idx, &tab)| {
                                SidebarMenuItem::new(tab.label())
                                    .text_lg()
                                    .map(|this| if idx == 0 { this.mt_neg_2() } else { this.mt_1() })
                                    .active(active_panel_tab == tab)
                                    .icon(tab.icon())
                                    .on_click(window.listener_for(&self.side_panel, move |side_panel, _, _, _| {
                                        side_panel.open_tab(tab);
                                    }))
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
                            .with_state(&self.resizable)
                            .when(self.side_panel_expanded, |this| {
                                this.child(
                                    resizable_panel()
                                        .size(DEFAULT_SIDE_PANEL_WIDTH)
                                        .child(SidePanel::new(&self.side_panel)),
                                )
                            })
                            .child(resizable_panel().map(|this| {
                                this.child(if AppState::connection(cx).is_some() {
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
                    .bg(cx.theme().sidebar)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .when_some(AppState::connection(cx), |this, connection| {
                        this.when_some(connection.color(cx), |this, color| this.bg(color))
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .items_center()
                                    .justify_between()
                                    .child(SharedString::from(connection.name(cx)))
                                    .child(
                                        div().track_focus(&self.color_picker_focus_handle).child(
                                            ColorPicker::new(&self.color_picker).icon(IconName::Palette).small(),
                                        ),
                                    ),
                            )
                    }),
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
            .dropdown_menu(|menu, _, _| {
                menu.menu("Connections", Box::new(ApplicationAction::OpenConnectionDialog))
                    .separator()
                    .menu("Settings", Box::new(ApplicationAction::OpenSettings))
            }),
    )
}
