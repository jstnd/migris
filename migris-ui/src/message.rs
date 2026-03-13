use iced::widget::pane_grid;

use crate::widgets::tree::TreeItemId;

#[derive(Debug, Clone)]
pub enum Message {
    PanelResized(pane_grid::ResizeEvent),
    TreeItemSelected(TreeItemId),
    TreeItemToggled(TreeItemId),
}
