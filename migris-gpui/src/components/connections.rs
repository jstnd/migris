use std::collections::HashMap;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    Sizable, WindowExt,
    button::{Button, ButtonVariants},
    dialog::{Dialog, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState, MaskPattern},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    select::{Select, SelectState},
    tree::{self, TreeItem, TreeState},
    v_flex,
};
use migris::connection::{ConnectionOptions, MySqlOptions};

use crate::{
    components::{
        icon::{Icon, IconName},
        labeled,
    },
    connections::{ConnectionFolderId, ConnectionId, ConnectionManager},
    event::AppEvent,
    shared,
    state::AppState,
};

pub fn connection_dialog(dialog: Dialog, window: &mut Window, cx: &mut App) -> Dialog {
    let dialog_state = &AppState::global(cx).connection_dialog_state;

    dialog
        .w(shared::DIALOG_WIDTH)
        .h(shared::DIALOG_HEIGHT)
        .title("Connections")
        .child(
            h_resizable("connections-dialog-panels")
                .child(
                    resizable_panel().size(shared::DIALOG_WIDTH * 0.3).child(
                        v_flex()
                            .gap_1()
                            .p_3()
                            .size_full()
                            .items_center()
                            .child(
                                Input::new(&dialog_state.read(cx).search_input)
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
                                            .ghost()
                                            .small()
                                            .on_click(|_, _, _| {}),
                                    )
                                    .child(
                                        Button::new("button-new-connection")
                                            .icon(Icon::new(cx, IconName::Plus))
                                            .tooltip("New Connection")
                                            .ghost()
                                            .small()
                                            .on_click(|_, _, _| {}),
                                    ),
                            )
                            .child(tree::tree(&dialog_state.read(cx).tree, {
                                let dialog_state = dialog_state.clone();
                                move |idx, entry, _, window, cx| {
                                    let manager = ConnectionManager::global(cx);
                                    let connection = manager.try_connection(&entry.item().id);
                                    let connection_id =
                                        connection.map(|connection| connection.id());
                                    let folder = manager.try_folder(&entry.item().id);

                                    ListItem::new(idx)
                                        .px_1()
                                        .py_0()
                                        .text_sm()
                                        .child(
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
                                        .on_click(window.listener_for(
                                            &dialog_state,
                                            move |state, _, window, cx| {
                                                state.editor.update(cx, |editor, cx| {
                                                    if let Some(id) = connection_id {
                                                        editor.open(window, cx, id);
                                                    } else {
                                                        editor.close();
                                                    }
                                                });
                                            },
                                        ))
                                }
                            })),
                    ),
                )
                .child(
                    resizable_panel().child(ConnectionEditor::new(&dialog_state.read(cx).editor)),
                ),
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
                            .icon(IconName::Save)
                            .label("Save")
                            .compact()
                            .primary()
                            .on_click(|_, _, cx| {
                                ConnectionManager::global_mut(cx).save();
                            }),
                    )
                    .child(
                        Button::new("connections-open")
                            .label("Open")
                            .primary()
                            .on_click(window.listener_for(dialog_state, |_, _, window, cx| {
                                window.close_dialog(cx);
                            })),
                    ),
            ),
        )
        .on_close(|_, _, cx| {
            // Revert any changes to the config.
            //ConnectionManager::global_mut(cx).revert();
        })
}

pub struct ConnectionDialogState {
    /// The state for the connection editor.
    editor: Entity<ConnectionEditorState>,

    /// The state for the connection search input.
    search_input: Entity<InputState>,

    /// The state for the connection tree.
    tree: Entity<TreeState>,
}

impl EventEmitter<AppEvent> for ConnectionDialogState {}

impl ConnectionDialogState {
    /// Creates a new [`ConnectionDialogState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| ConnectionEditorState::new(window, cx));
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(shared::SEARCH_PLACEHOLDER));
        let tree = cx.new(|cx| TreeState::new(cx));
        let mut state = Self {
            editor,
            search_input,
            tree,
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
        self.tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
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

struct ConnectionEditorState {
    ///
    name_input: Entity<InputState>,

    ///
    type_select: Entity<SelectState<Vec<SharedString>>>,

    /// The variant of the editor.
    ///
    /// This will be [`None`] if we do not have a connection selected to edit.
    variant: Option<ConnectionEditorVariant>,
}

impl ConnectionEditorState {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx));
        let type_select =
            cx.new(|cx| SelectState::new(vec![SharedString::from("test")], None, window, cx));

        Self {
            name_input,
            type_select,
            variant: None,
        }
    }

    fn close(&mut self) {
        self.variant = None;
    }

    fn open(&mut self, window: &mut Window, cx: &mut App, id: ConnectionId) {
        let connection = ConnectionManager::global(cx).connection(&id).clone();

        self.name_input.update(cx, |name_input, cx| {
            name_input.set_value(connection.name(), window, cx);
        });

        self.type_select.update(cx, |type_select, cx| {
            type_select.set_selected_value(&SharedString::from(""), window, cx);
        });

        self.variant = Some(match connection.options() {
            ConnectionOptions::MySql(options) => {
                let state = MySqlEditorState::new(window, cx, connection.id(), options);
                ConnectionEditorVariant::MySql(state)
            }
        });
    }
}

enum ConnectionEditorVariant {
    MySql(MySqlEditorState),
}

#[derive(IntoElement)]
struct ConnectionEditor {
    ///
    state: Entity<ConnectionEditorState>,
}

impl ConnectionEditor {
    fn new(state: &Entity<ConnectionEditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for ConnectionEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        v_flex().gap_1().p_3().w_full().map(|this| {
            if let Some(variant) = &state.variant {
                this.child(labeled("Name", Input::new(&state.name_input)))
                    .child(labeled("Connection Type", Select::new(&state.type_select)))
                    .map(|this| match variant {
                        ConnectionEditorVariant::MySql(state) => this
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(labeled("Host", Input::new(&state.host_input)))
                                    .child(
                                        h_flex()
                                            .w_1_4()
                                            .child(labeled("Port", Input::new(&state.port_input))),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(labeled("User", Input::new(&state.user_input)))
                                    .child(labeled(
                                        "Password",
                                        Input::new(&state.password_input).mask_toggle(),
                                    )),
                            ),
                    })
            } else {
                this
            }
        })
    }
}

struct MySqlEditorState {
    ///
    id: ConnectionId,

    ///
    host_input: Entity<InputState>,

    ///
    port_input: Entity<InputState>,

    ///
    user_input: Entity<InputState>,

    ///
    password_input: Entity<InputState>,
}

impl MySqlEditorState {
    fn new(window: &mut Window, cx: &mut App, id: ConnectionId, options: &MySqlOptions) -> Self {
        let host_input = cx.new(|cx| InputState::new(window, cx).default_value(&options.host));
        let port_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(options.port.to_string())
                .mask_pattern(MaskPattern::Number {
                    separator: None,
                    fraction: None,
                })
        });

        let user_input = cx.new(|cx| InputState::new(window, cx).default_value(&options.user));
        let password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(&options.password)
                .masked(true)
        });

        Self {
            id,
            host_input,
            port_input,
            user_input,
            password_input,
        }
    }

    fn id(&self) -> ConnectionId {
        self.id
    }

    fn options(&self, cx: &App) -> ConnectionOptions {
        let options = MySqlOptions {
            host: self.host_input.read(cx).value().to_string(),
            port: 0,
            user: self.user_input.read(cx).value().to_string(),
            password: self.password_input.read(cx).value().to_string(),
        };

        ConnectionOptions::MySql(options)
    }
}
