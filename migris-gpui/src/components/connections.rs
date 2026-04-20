use std::collections::HashMap;

use gpui::{App, AppContext, Context, Entity, ParentElement, Styled, Window};
use gpui_component::{
    dialog::Dialog,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    tree::{self, TreeItem, TreeState},
};

use crate::{
    connections::{ConnectionFolderId, ConnectionId, ConnectionManager},
    shared,
    state::AppState,
};

pub fn connection_dialog(dialog: Dialog, _: &mut Window, cx: &mut App) -> Dialog {
    let dialog_state = &AppState::global(cx).connection_dialog_state;

    dialog
        .w(shared::DIALOG_WIDTH)
        .h(shared::DIALOG_HEIGHT)
        .title("Connections")
        .child(
            h_resizable("connections-dialog-panels").child(resizable_panel().child(tree::tree(
                &dialog_state.read(cx).tree_state,
                |idx, entry, _, _, _| ListItem::new(idx).child(entry.item().label.clone()),
            ))),
        )
}

pub struct ConnectionDialogState {
    tree_state: Entity<TreeState>,
}

impl ConnectionDialogState {
    /// Creates a new [`ConnectionDialogState`].
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let mut state = Self { tree_state };
        state.load_tree(cx);
        state
    }

    fn load_tree(&mut self, cx: &mut Context<Self>) {
        let manager = ConnectionManager::global(cx);

        //
        let mut folders: HashMap<Option<ConnectionFolderId>, Vec<ConnectionFolderId>> =
            HashMap::new();

        //
        let mut connections_by_folder: HashMap<Option<ConnectionFolderId>, Vec<ConnectionId>> =
            HashMap::new();

        //
        for folder in manager.folders() {
            folders
                .entry(folder.parent())
                .or_default()
                .push(folder.id());
        }

        //
        for connection in manager.connections() {
            connections_by_folder
                .entry(connection.folder())
                .or_default()
                .push(connection.id());
        }

        let items = Self::build_tree_items(manager, None, &folders, &connections_by_folder);
        self.tree_state.update(cx, |tree_state, cx| {
            tree_state.set_items(items, cx);
        });
    }

    fn build_tree_items(
        manager: &ConnectionManager,
        folder_id: Option<ConnectionFolderId>,
        folders: &HashMap<Option<ConnectionFolderId>, Vec<ConnectionFolderId>>,
        connections_by_folder: &HashMap<Option<ConnectionFolderId>, Vec<ConnectionId>>,
    ) -> Vec<TreeItem> {
        let mut items = Vec::new();

        //
        if let Some(children_folders) = folders.get(&folder_id) {
            let mut folder_items = Vec::new();

            for id in children_folders {
                let folder = manager.folder(id);
                let item = TreeItem::new(id.to_string(), folder.name()).children(
                    Self::build_tree_items(manager, Some(*id), folders, connections_by_folder),
                );
                folder_items.push(item);
            }

            folder_items.sort_unstable_by(|a, b| a.label.cmp(&b.label));
            items.extend(folder_items);
        }

        //
        if let Some(children_connections) = connections_by_folder.get(&folder_id) {
            let mut connection_items = Vec::new();

            for id in children_connections {
                let connection = manager.connection(id);
                let item = TreeItem::new(id.to_string(), connection.name());
                connection_items.push(item);
            }

            connection_items.sort_unstable_by(|a, b| a.label.cmp(&b.label));
            items.extend(connection_items);
        }

        items
    }
}
