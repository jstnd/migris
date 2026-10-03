use std::sync::Arc;

use gpui_kit::{
    App, AppContext, BorrowAppContext, Entity, FocusHandle, Global, Window,
    component::{Theme, ThemeMode},
};

use crate::{
    components::connection_dialog::ConnectionDialogState, database::Database, settings::SettingsManager,
    types::OpenConnection,
};

pub struct AppState {
    /// The currently open connection.
    pub connection: Option<OpenConnection>,

    /// The state for the application's connection dialog.
    ///
    /// This is stored here to be available globally so that the dialog can be opened from anywhere.
    pub connection_dialog: Entity<ConnectionDialogState>,

    /// The database for the application.
    pub database: Arc<Database>,

    /// The primary focus handle for the application.
    pub focus_handle: FocusHandle,

    /// The current system theme mode.
    pub system_theme_mode: ThemeMode,
}

impl Global for AppState {}

impl AppState {
    /// Creates a new [`AppState`].
    pub fn new(window: &mut Window, cx: &mut App, database: Arc<Database>) -> Self {
        let connection_dialog = cx.new(|cx| ConnectionDialogState::new(window, cx));
        Self::init(window);

        Self {
            connection: None,
            connection_dialog,
            database,
            focus_handle: cx.focus_handle(),
            system_theme_mode: ThemeMode::from(window.appearance()),
        }
    }

    /// Initializes functionality needed for the global [`AppState`].
    fn init(window: &mut Window) {
        // Listen to changes in the system theme; this is needed for
        // when the user has the system app theme mode selected.
        window
            .observe_window_appearance(|window, cx| {
                cx.update_global(|state: &mut AppState, cx| {
                    state.system_theme_mode = ThemeMode::from(window.appearance());

                    if SettingsManager::app_theme_mode(cx).is_system() {
                        Theme::change(state.system_theme_mode, None, cx);
                        cx.refresh_windows();
                    }
                });
            })
            .detach();
    }

    /// Returns a reference to the global [`AppState`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns a mutable reference to the global [`AppState`].
    fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Returns a reference to the open connection, if one exists.
    pub fn connection(cx: &App) -> Option<&OpenConnection> {
        Self::global(cx).connection.as_ref()
    }

    /// Returns a reference to the open connection, panicking if one does not exist.
    pub fn connection_unchecked(cx: &App) -> &OpenConnection {
        Self::global(cx).connection.as_ref().unwrap()
    }

    /// Returns a cloned pointer instance of the application's database.
    pub fn database(cx: &App) -> Arc<Database> {
        Self::global(cx).database.clone()
    }

    /// Returns a reference to the primary focus handle for the application.
    pub fn handle(cx: &App) -> &FocusHandle {
        &Self::global(cx).focus_handle
    }

    /// Loads needed information for the connection dialog.
    pub fn load_connection_dialog(&self, cx: &mut App) {
        self.connection_dialog.update(cx, |connection_dialog, cx| {
            connection_dialog.load_tree(cx);
        });
    }

    /// Sets the open connection for the application.
    pub fn set_connection(cx: &mut App, connection: OpenConnection) {
        Self::global_mut(cx).connection = Some(connection);
    }
}
