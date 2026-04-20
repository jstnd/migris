use gpui::{App, AppContext, BorrowAppContext, Entity, Global, Window};
use gpui_component::{Theme, ThemeMode};

use crate::{components::connections::ConnectionDialogState, settings::AppSettings};

pub struct AppState {
    /// The state for the application's connection dialog.
    ///
    /// This is stored here to be available globally so that the dialog can be opened from anywhere.
    pub connection_dialog_state: Entity<ConnectionDialogState>,

    /// The current system theme mode.
    pub system_theme_mode: ThemeMode,
}

impl Global for AppState {}

impl AppState {
    /// Creates a new [`AppState`].
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let connection_dialog_state = cx.new(ConnectionDialogState::new);
        Self::init(window);

        Self {
            connection_dialog_state,
            system_theme_mode: ThemeMode::default(),
        }
    }

    /// Returns a reference to the global [`AppState`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Initializes functionality needed for the global [`AppState`].
    fn init(window: &mut Window) {
        // Listen to changes in the system theme; this is needed for
        // when the user has the system app theme mode selected.
        window
            .observe_window_appearance(|window, cx| {
                cx.update_global(|state: &mut AppState, cx| {
                    state.system_theme_mode = ThemeMode::from(window.appearance());

                    if AppSettings::global(cx).theme_mode.is_system() {
                        Theme::change(state.system_theme_mode, None, cx);
                        cx.refresh_windows();
                    }
                });
            })
            .detach();
    }
}
