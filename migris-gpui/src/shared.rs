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

pub mod colors {
    use gpui_kit::{
        App, Hsla,
        component::{ActiveTheme, try_parse_color},
    };

    /// Returns the foreground color for the given background HSLA color.
    pub fn foreground_for_hsla(cx: &App, hsla: Hsla) -> Hsla {
        let luminance = relative_luminance(hsla);

        // The 0.179 value here is derived from the contrast-ratio formula defined by W3C.
        //
        // https://dev.to/louis7/how-to-choose-the-font-color-based-on-the-background-color-402a
        // https://www.w3.org/TR/WCAG21/#dfn-contrast-ratio
        let color = if luminance > 0.179 {
            &cx.theme().light_theme.colors.foreground
        } else {
            &cx.theme().dark_theme.colors.foreground
        };

        if let Some(color) = color {
            try_parse_color(color).unwrap_or(cx.theme().foreground)
        } else {
            cx.theme().foreground
        }
    }

    /// Returns the linearized value calculated from the given RGB value.
    ///
    /// https://en.wikipedia.org/wiki/SRGB#Transfer_function_(%22gamma%22)
    fn linearize(value: f32) -> f32 {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Returns the relative luminance of the given HSLA.
    ///
    /// https://en.wikipedia.org/wiki/Relative_luminance
    pub fn relative_luminance(hsla: Hsla) -> f32 {
        let rgb = hsla.to_rgb();
        0.2126 * linearize(rgb.r) + 0.7152 * linearize(rgb.g) + 0.0722 * linearize(rgb.b)
    }
}
