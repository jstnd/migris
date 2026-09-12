use std::sync::Arc;

use anyhow::Result;
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

/// Creates a database driver from the given connection.
pub async fn create_driver(connection: &Connection) -> Result<Arc<dyn Driver>> {
    let connection_string = connection.connection_string();
    Ok(match connection.kind {
        ConnectionKind::MySql => Arc::new(MySqlConnection::new(connection_string).await?),
        ConnectionKind::Sqlite => Arc::new(SqliteConnection::new(&connection_string).await?),
    })
}
