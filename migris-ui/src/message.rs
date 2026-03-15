use std::sync::Arc;

use iced::widget::pane_grid;
use migris::driver::{Driver, Entity};

use crate::widgets::tree::TreeItemId;

#[derive(Debug, Clone)]
pub enum Message {
    /// Fires when a connection is added to the list of active connections.
    ConnectionAdded,

    /// Fires when a connection has successfully loaded.
    ConnectionLoaded(Arc<dyn Driver>, Vec<Entity>),

    PanelResized(pane_grid::ResizeEvent),
    TreeItemSelected(TreeItemId),
    TreeItemToggled(TreeItemId),

    /// Fires when an error occurs somewhere in the application.
    ErrorEncountered(String),
}
