use std::collections::HashMap;

use gpui::{
    App, AppContext, Context, Entity, ParentElement, Styled, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    WindowExt,
    button::{Button, ButtonVariants},
    dialog::{Dialog, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    tree::{self, TreeItem, TreeState},
    v_flex,
};

use crate::{
    components::icon::{Icon, IconName},
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
            h_resizable("connections-dialog-panels")
                .child(
                    resizable_panel().size(shared::DIALOG_WIDTH * 0.30).child(
                        v_flex()
                            .gap_1()
                            .p_3()
                            .size_full()
                            .items_center()
                            .child(
                                Input::new(&dialog_state.read(cx).search_state)
                                    .cleanable(true)
                                    .prefix(Icon::new(cx, IconName::Search)),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_end()
                                    .child(
                                        Button::new("button-new-folder")
                                            .icon(Icon::new(cx, IconName::FolderPlus))
                                            .tooltip("New Folder")
                                            .compact()
                                            .ghost()
                                            .on_click(|_, _, _| {}),
                                    )
                                    .child(
                                        Button::new("button-new-connection")
                                            .icon(Icon::new(cx, IconName::Plus))
                                            .tooltip("New Connection")
                                            .compact()
                                            .ghost()
                                            .on_click(|_, _, _| {}),
                                    ),
                            )
                            .child(tree::tree(
                                &dialog_state.read(cx).tree_state,
                                |idx, entry, _, _, cx| {
                                    let manager = ConnectionManager::global(cx);
                                    let connection = manager.try_connection(&entry.item().id);
                                    let folder = manager.try_folder(&entry.item().id);

                                    ListItem::new(idx).p_0().px_1().text_sm().child(
                                        h_flex()
                                            .gap_1()
                                            .pl(px(18.0) * entry.depth())
                                            .when_some(connection, |this, _| {
                                                // TODO: change icon to match connection type
                                                this.child(Icon::new(cx, IconName::Database))
                                            })
                                            .when(folder.is_some(), |this| {
                                                this.child(Icon::new(
                                                    cx,
                                                    if entry.is_expanded() {
                                                        IconName::FolderOpen
                                                    } else {
                                                        IconName::Folder
                                                    },
                                                ))
                                            })
                                            .child(entry.item().label.clone()),
                                    )
                                },
                            )),
                    ),
                )
                .child(resizable_panel().child(v_flex().size_full().child("TEST"))),
        )
        .footer(
            DialogFooter::new().child(
                h_flex()
                    .gap_2()
                    .child(
                        DialogClose::new().child(Button::new("connections-cancel").label("Cancel")),
                    )
                    .child(
                        Button::new("connections-save")
                            .label("Save")
                            .primary()
                            .on_click(|_, _, cx| {
                                ConnectionManager::global_mut(cx).save();
                            }),
                    )
                    .child(
                        Button::new("connections-open")
                            .label("Open")
                            .primary()
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
            ),
        )
        .on_close(|_, _, cx| {
            // Revert any changes to the config.
            ConnectionManager::global_mut(cx).revert();
        })
}

pub struct ConnectionDialogState {
    /// The state for the connection search input.
    search_state: Entity<InputState>,

    /// The state for the connection tree.
    tree_state: Entity<TreeState>,
}

impl ConnectionDialogState {
    /// Creates a new [`ConnectionDialogState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state =
            cx.new(|cx| InputState::new(window, cx).placeholder(shared::SEARCH_PLACEHOLDER));
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let mut state = Self {
            search_state,
            tree_state,
        };

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
