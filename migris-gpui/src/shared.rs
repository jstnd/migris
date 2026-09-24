use std::{path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};
use directories::BaseDirs;
use gpui_kit::{Pixels, px};
use migris::{
    drivers::{ConnectionKind, Driver},
    mysql::MySqlConnection,
    sqlite::SqliteConnection,
};

use crate::connections::Connection;

/// The application name.
pub const APPLICATION_NAME: &str = "Migris";

/// The application name in lowercase format.
pub const APPLICATION_NAME_LOWER: &str = "migris";

/// The width of primary dialogs (e.g. connections, settings).
pub const DIALOG_WIDTH: Pixels = px(800.0);

/// The height of primary dialogs (e.g. connections, settings).
pub const DIALOG_HEIGHT: Pixels = px(600.0);

/// The placeholder text for search input fields.
pub const SEARCH_PLACEHOLDER: &str = "Search...";

/// Returns the path of the application's folder within the user's config directory.
pub fn config_dir() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .ok_or(anyhow!("failed to create BaseDirs struct"))?
        .config_dir()
        .join(APPLICATION_NAME))
}

/// Creates the application's folder within the user's config directory.
pub fn create_config_dir() -> Result<()> {
    std::fs::create_dir_all(config_dir()?)?;
    Ok(())
}

/// Creates a database driver from the given connection.
pub async fn create_driver(connection: &Connection) -> Result<Arc<dyn Driver>> {
    let connection_string = connection.connection_string();
    Ok(match connection.kind {
        ConnectionKind::MySql => Arc::new(MySqlConnection::new(connection_string).await?),
        ConnectionKind::Sqlite => Arc::new(SqliteConnection::new(&connection_string).await?),
    })
}

/// Formats the given milliseconds into a more human-readable string.
pub fn format_ms(ms: u64) -> String {
    if ms == 0 {
        "<1ms".to_string()
    } else if ms < 1000 {
        format!("{}ms", ms)
    } else {
        let seconds = ms as f32 / 1000.0;
        format!("{:.3}s", seconds)
    }
}
