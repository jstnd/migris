use std::{path::PathBuf, sync::Arc, time::Duration};

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

/// The placeholder text for filter input fields.
pub const FILTER_PLACEHOLDER: &str = "Filter...";

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

/// Formats the given number to contain commas.
pub fn format_number(number: u64) -> String {
    number
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

/// Returns the given duration in a timer format (e.g. 00:00.0).
pub fn format_timer(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let tenths = duration.subsec_millis() / 100;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{tenths}")
    } else {
        format!("{minutes:02}:{seconds:02}.{tenths}")
    }
}
