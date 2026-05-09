use std::{collections::HashSet, time::Duration};

use gpui::{
    Action, App, AppContext, ClickEvent, Context, Div, Entity, InteractiveElement, IntoElement,
    KeystrokeEvent, MouseButton, ParentElement, Pixels, Render, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    dialog::{Dialog, DialogFooter},
    h_flex,
    input::{Input, InputEvent, InputState, MaskPattern},
    list::ListItem,
    progress::ProgressCircle,
    resizable::{h_resizable, resizable_panel},
    select::SelectState,
    tree::{self, TreeItem, TreeState},
    v_flex,
};
use migris::connection::{ConnectionOptions, MySqlOptions};

use crate::{
    components::{
        self,
        icon::{Icon, IconName},
        labeled, text_ellipsis,
    },
    connections::{
        Connection, ConnectionFolder, ConnectionFolderId, ConnectionId, ConnectionManager,
    },
    events::{Event, EventManager, EventVariant},
    shared,
    state::AppState,
};

const CONNECTION_DIALOG: &str = "CONNECTION_DIALOG";
const DRAG_CONTAINER_WIDTH: Pixels = px(150.0);

pub fn connection_dialog(dialog: Dialog, window: &mut Window, cx: &mut App) -> Dialog {
    let state = &AppState::global(cx).connection_dialog;
    let is_editor_empty = state.read(cx).is_editor_empty(cx);
    let is_opening = state.read(cx).opening;

    dialog
        .w(shared::DIALOG_WIDTH)
        .h(shared::DIALOG_HEIGHT)
        .title("Connections")
        .child(
            h_resizable("connections-dialog-panels")
                .child(
                    resizable_panel().size(shared::DIALOG_WIDTH * 0.3).child(
                        v_flex()
                            .key_context(CONNECTION_DIALOG)
                            .gap_1()
                            .size_full()
                            .items_center()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .pt_3()
                                    .px_3()
                                    .w_full()
                                    .child(
                                        Input::new(&state.read(cx).search_input)
                                            .cleanable(true)
                                            .prefix(Icon::new(cx, IconName::Search)),
                                    )
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .justify_end()
                                            .child(
                                                Button::new("button-new-connection")
                                                    .icon(Icon::new(cx, IconName::Plus))
                                                    .tooltip("New Connection")
                                                    .ghost()
                                                    .small()
                                                    .on_click(window.listener_for(
                                                        state,
                                                        |state, _, window, cx| {
                                                            state.handle_action(
                                                        window,
                                                        cx,
                                                        &ConnectionDialogAction::AddConnection(
                                                            state.selected_parent(cx),
                                                        ),
                                                    );
                                                        },
                                                    )),
                                            )
                                            .child(
                                                Button::new("button-new-folder")
                                                    .icon(Icon::new(cx, IconName::FolderPlus))
                                                    .tooltip("New Folder")
                                                    .ghost()
                                                    .small()
                                                    .on_click(window.listener_for(
                                                        state,
                                                        |state, _, window, cx| {
                                                            state.handle_action(
                                                                window,
                                                                cx,
                                                                &ConnectionDialogAction::AddFolder(
                                                                    state.selected_parent(cx),
                                                                ),
                                                            );
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .child(connection_tree(state.clone(), window, cx))
                            .on_action(window.listener_for(state, |state, action, window, cx| {
                                state.handle_action(window, cx, action);
                            })),
                    ),
                )
                .child(resizable_panel().child(ConnectionEditor::new(&state.read(cx).editor))),
        )
        .footer(
            DialogFooter::new().child(
                h_flex()
                    .gap_2()
                    .child(Button::new("connections-cancel").label("Cancel").on_click(
                        window.listener_for(state, |state, _, window, cx| {
                            state.reset(cx);
                            window.close_dialog(cx);
                        }),
                    ))
                    .child(
                        Button::new("connections-save")
                            .icon(IconName::Save)
                            .label("Save")
                            .compact()
                            .primary()
                            .disabled(is_editor_empty || is_opening)
                            .on_click(window.listener_for(state, |state, _, window, cx| {
                                state.save_inline_editor(window, cx);
                                state.save_editor(cx);
                            })),
                    )
                    .child(
                        Button::new("connections-open")
                            .label("Open")
                            .compact()
                            .primary()
                            .disabled(is_editor_empty)
                            .when(is_opening, |this| {
                                this.icon(
                                    ProgressCircle::new("open-progress")
                                        .color(cx.theme().button_primary_foreground)
                                        .loading(true),
                                )
                            })
                            .on_click(window.listener_for(state, move |state, _, window, cx| {
                                state.open_connection(window, cx);
                            })),
                    ),
            ),
        )
        .on_close(window.listener_for(state, |state, _, _, cx| {
            state.reset(cx);
        }))
        .on_ok({
            let state = state.clone();
            move |_, window, cx| {
                state.update(cx, |state, cx| {
                    // We do not want to open the connection if this closure was called as a result of pressing the Enter key inside the inline editor.
                    //
                    // The Enter key is used for both saving the inline editor and confirming the entire dialog, and we want these events to be mutually exclusive.
                    if state.inline_editor_id.is_none() {
                        state.open_connection(window, cx);
                    }
                });

                false
            }
        })
}

fn connection_tree(
    state: Entity<ConnectionDialogState>,
    window: &mut Window,
    cx: &App,
) -> impl IntoElement {
    div()
        .id("connection-tree")
        .size_full()
        .child(
            tree::tree(&state.read(cx).tree, {
                let state = state.clone();
                move |idx, entry, _, window, cx| {
                    let manager = ConnectionManager::global(cx);
                    let connection = manager.try_connection(&entry.item().id);
                    let connection_id = connection.map(|c| c.id());
                    let folder = manager.try_folder(&entry.item().id);
                    let folder_id = folder.map(|f| f.id());

                    let is_inline_editing = state.read(cx).is_inline_editing(&entry.item().id);

                    ListItem::new(idx)
                        .ml_3()
                        .mr_4()
                        .p_0()
                        .text_sm()
                        .child(
                            h_flex()
                                .id(("connection-item", idx))
                                .gap_1()
                                .px_1()
                                .when(entry.depth() > 0, |this| this.pl(px(18.0) * entry.depth()))
                                .when_some(connection, |this, connection| {
                                    let folder = connection.folder();

                                    // TODO: change icon to match connection type
                                    this.child(Icon::new(cx, IconName::Database))
                                        .when(!is_inline_editing, |this| {
                                            this.on_drag(
                                                DragConnection {
                                                    id: connection.id(),
                                                    offset: None,
                                                },
                                                |drag, offset, _, cx| {
                                                    let mut drag = *drag;
                                                    drag.offset = Some(offset.x);
                                                    cx.new(|_| drag)
                                                },
                                            )
                                        })
                                        .drag_over(|this, _: &DragConnection, _, cx| {
                                            this.border_1()
                                                .border_color(cx.theme().list_active_border)
                                        })
                                        .drag_over(|this, _: &DragFolder, _, cx| {
                                            this.border_1()
                                                .border_color(cx.theme().list_active_border)
                                        })
                                        .on_drop(window.listener_for(
                                            &state,
                                            move |state, drag: &DragConnection, _, cx| {
                                                state.move_connection(cx, &drag.id, folder);
                                            },
                                        ))
                                        .on_drop(window.listener_for(
                                            &state,
                                            move |state, drag: &DragFolder, _, cx| {
                                                state.move_folder(cx, &drag.id, folder);
                                            },
                                        ))
                                })
                                .when_some(folder, |this, folder| {
                                    let id = folder.id();
                                    let is_expanded = state.read(cx).is_expanded(&folder.id());

                                    this.child(Icon::new(
                                        cx,
                                        if is_expanded {
                                            IconName::FolderOpen
                                        } else {
                                            IconName::Folder
                                        },
                                    ))
                                    .when(!is_inline_editing, |this| {
                                        this.on_drag(
                                            DragFolder {
                                                id: folder.id(),
                                                is_expanded,
                                                offset: None,
                                            },
                                            |drag, offset, _, cx| {
                                                let mut drag = *drag;
                                                drag.offset = Some(offset.x);
                                                cx.new(|_| drag)
                                            },
                                        )
                                    })
                                    .drag_over(|this, _: &DragConnection, _, cx| {
                                        this.border_1().border_color(cx.theme().list_active_border)
                                    })
                                    .drag_over(|this, _: &DragFolder, _, cx| {
                                        this.border_1().border_color(cx.theme().list_active_border)
                                    })
                                    .on_drop(window.listener_for(
                                        &state,
                                        move |state, drag: &DragConnection, _, cx| {
                                            state.move_connection(cx, &drag.id, Some(id));
                                        },
                                    ))
                                    .on_drop(
                                        window.listener_for(
                                            &state,
                                            move |state, drag: &DragFolder, _, cx| {
                                                state.move_folder(cx, &drag.id, Some(id));
                                            },
                                        ),
                                    )
                                })
                                .child({
                                    if state.read(cx).is_inline_editing(&entry.item().id) {
                                        Input::new(&state.read(cx).inline_name_input)
                                            .p_0()
                                            .appearance(false)
                                            .small()
                                            .into_any_element()
                                    } else {
                                        text_ellipsis(entry.item().label.clone()).into_any_element()
                                    }
                                }),
                        )
                        .on_click({
                            let item = entry.item().clone();
                            window.listener_for(
                                &state,
                                move |state, event: &ClickEvent, window, cx| {
                                    // We do not want to perform any click event handling if we are inline editing this item.
                                    if state.is_inline_editing(&item.id) {
                                        return;
                                    }

                                    // Set the selected item after click.
                                    state.tree.update(cx, |tree, cx| {
                                        tree.focus(window, cx);
                                        tree.set_selected_item(Some(&item), cx);
                                    });

                                    if event.click_count() >= 2 {
                                        if connection_id.is_some() {
                                            // Open connection on double-click.
                                            state.open_connection(window, cx);
                                        } else if let Some(id) = folder_id {
                                            // Open folder in inline editor on double-click.
                                            state.open_inline_editor(window, cx, id.to_string());
                                        }

                                        return;
                                    } else if let Some(id) = folder_id {
                                        state.toggle_expand(cx, id);
                                    }

                                    state.open_editor(window, cx, connection_id);
                                },
                            )
                        })
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            // The underlying tree component uses the `on_mouse_down` event to handle things such as item expansion.
                            // We stop that event from firing using `stop_propagation` here to give us more control over behavior resulting from mouse events.
                            cx.stop_propagation();
                        })
                }
            })
            .context_menu(|_, entry, menu, _, cx| {
                let manager = ConnectionManager::global(cx);
                let connection_id = manager
                    .try_connection(&entry.item().id)
                    .map(|connection| connection.id());
                let folder_id = manager
                    .try_folder(&entry.item().id)
                    .map(|folder| folder.id());

                if let Some(id) = connection_id {
                    menu.menu_with_icon(
                        "Rename",
                        Icon::primary(cx, IconName::TextCursorInput),
                        Box::new(ConnectionDialogAction::RenameItem(id.to_string())),
                    )
                    .menu_with_icon(
                        "Duplicate",
                        Icon::primary(cx, IconName::Copy),
                        Box::new(ConnectionDialogAction::DuplicateConnection(id)),
                    )
                    .menu_with_icon(
                        "Delete",
                        Icon::danger(cx, IconName::Trash),
                        Box::new(ConnectionDialogAction::DeleteConnection(id)),
                    )
                } else if let Some(id) = folder_id {
                    menu.menu_with_icon(
                        "Rename",
                        Icon::primary(cx, IconName::TextCursorInput),
                        Box::new(ConnectionDialogAction::RenameItem(id.to_string())),
                    )
                    .menu_with_icon(
                        "Duplicate",
                        Icon::primary(cx, IconName::Copy),
                        Box::new(ConnectionDialogAction::DuplicateFolder(id)),
                    )
                    .menu_with_icon(
                        "Delete",
                        Icon::danger(cx, IconName::Trash),
                        Box::new(ConnectionDialogAction::DeleteFolder(id)),
                    )
                    .separator()
                    .menu_with_icon(
                        "New Connection",
                        Icon::primary(cx, IconName::Plus),
                        Box::new(ConnectionDialogAction::AddConnection(Some(id))),
                    )
                    .menu_with_icon(
                        "New Folder",
                        Icon::primary(cx, IconName::FolderPlus),
                        Box::new(ConnectionDialogAction::AddFolder(Some(id))),
                    )
                } else {
                    menu
                }
            }),
        )
        .on_drop(
            window.listener_for(&state, |state, drag: &DragConnection, _, cx| {
                state.move_connection(cx, &drag.id, None);
            }),
        )
        .on_drop(
            window.listener_for(&state, |state, drag: &DragFolder, _, cx| {
                state.move_folder(cx, &drag.id, None);
            }),
        )
}

fn drag_container(cx: &App) -> Div {
    h_flex()
        .gap_1()
        .px_1()
        .w(DRAG_CONTAINER_WIDTH)
        .bg(cx.theme().background)
        .border_1()
        .border_color(cx.theme().border)
        .opacity(0.9)
        .shadow_md()
        .text_color(cx.theme().muted_foreground)
        .text_sm()
}

#[derive(Clone, Copy)]
struct DragConnection {
    /// The id of the connection being dragged.
    id: ConnectionId,

    /// The position offset of where the connection was dragged from.
    offset: Option<Pixels>,
}

impl Render for DragConnection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let connection = ConnectionManager::global(cx).connection(&self.id);

        div()
            .when_some(self.offset, |this, offset| {
                this.pl(offset - (DRAG_CONTAINER_WIDTH / 2.0))
            })
            .child(
                drag_container(cx)
                    .child(Icon::new(cx, IconName::Database))
                    .child(text_ellipsis(connection.name())),
            )
    }
}

#[derive(Clone, Copy)]
struct DragFolder {
    /// The id of the folder being dragged.
    id: ConnectionFolderId,

    /// Whether the folder is expanded.
    is_expanded: bool,

    /// The position offset of where the folder was dragged from.
    offset: Option<Pixels>,
}

impl Render for DragFolder {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let folder = ConnectionManager::global(cx).folder(&self.id);

        div()
            .when_some(self.offset, |this, offset| {
                this.pl(offset - (DRAG_CONTAINER_WIDTH / 2.0))
            })
            .child(
                drag_container(cx)
                    .child(Icon::new(
                        cx,
                        if self.is_expanded {
                            IconName::FolderOpen
                        } else {
                            IconName::Folder
                        },
                    ))
                    .child(text_ellipsis(folder.name())),
            )
    }
}

#[derive(Action, Clone, PartialEq, Eq)]
#[action(no_json)]
enum ConnectionDialogAction {
    AddConnection(Option<ConnectionFolderId>),
    AddFolder(Option<ConnectionFolderId>),
    DeleteConnection(ConnectionId),
    DeleteFolder(ConnectionFolderId),
    DuplicateConnection(ConnectionId),
    DuplicateFolder(ConnectionFolderId),
    RenameItem(String),
}

/// The state used with the connection dialog.
pub struct ConnectionDialogState {
    /// The state for the connection editor.
    editor: Entity<ConnectionEditorState>,

    /// The state for the inline name input.
    inline_name_input: Entity<InputState>,

    /// The state for the connection search input.
    search_input: Entity<InputState>,

    /// The state for the connection tree.
    tree: Entity<TreeState>,

    /// The expanded folders.
    ///
    /// This is needed to persist expanded folders between actions that cause
    /// the tree to re-render, such as adding, editing, or deleting a connection.
    expanded: HashSet<ConnectionFolderId>,

    /// The id of the inline editor item.
    inline_editor_id: Option<SharedString>,

    /// Whether a connection is in the progress of opening.
    opening: bool,

    /// The subscriptions for the connection dialog.
    _subscriptions: Vec<Subscription>,
}

impl ConnectionDialogState {
    /// Creates a new [`ConnectionDialogState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| ConnectionEditorState::new(window, cx));
        let inline_name_input = cx.new(|cx| InputState::new(window, cx).context_menu(false));
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(shared::SEARCH_PLACEHOLDER));
        let tree = cx.new(|cx| TreeState::new(cx));

        let _subscriptions = Vec::from([
            cx.observe_keystrokes(|this, event, window, cx| {
                this.handle_keystroke(window, cx, event);
            }),
            cx.subscribe(&inline_name_input, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    let event = event.clone();
                    cx.spawn(async move |this, cx| {
                        // We wait here before saving the inline editor to allow other item click events to fire first
                        // in the scenario where we navigate from the inline editor item to another item in the tree.
                        //
                        // For example, if we click on a folder item with the inline editor active, we want that folder to
                        // toggle its expanded state first before saving, as saving will reload the tree and we want that
                        // new expanded state to take effect before the tree is reloaded to eliminate any odd rendering behavior.
                        cx.background_executor()
                            .timer(Duration::from_millis(100))
                            .await;

                        _ = this.update_in(cx, |this, window, cx| {
                            this.save_inline_editor(window, cx);

                            // Re-focus the tree if we're saving as a result of an event from the Enter key.
                            if let InputEvent::PressEnter { .. } = event {
                                this.tree.update(cx, |tree, cx| {
                                    tree.focus(window, cx);
                                });
                            }
                        });
                    })
                    .detach();
                }
            }),
            cx.subscribe(&search_input, |this, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.load_tree(cx);
                }
            }),
        ]);

        let state = Self {
            editor,
            inline_name_input,
            search_input,
            tree,
            expanded: HashSet::new(),
            inline_editor_id: None,
            opening: false,
            _subscriptions,
        };

        state.load_tree(cx);
        state
    }

    /// Handles actions originating from the connection dialog.
    fn handle_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: &ConnectionDialogAction,
    ) {
        match action {
            ConnectionDialogAction::AddConnection(folder) => self.add_connection(cx, *folder),
            ConnectionDialogAction::AddFolder(parent) => self.add_folder(cx, *parent),
            ConnectionDialogAction::DeleteConnection(id) => self.delete_connection(cx, id),
            ConnectionDialogAction::DeleteFolder(id) => self.delete_folder(cx, id),
            ConnectionDialogAction::DuplicateConnection(id) => self.duplicate_connection(cx, id),
            ConnectionDialogAction::DuplicateFolder(id) => self.duplicate_folder(cx, id),
            ConnectionDialogAction::RenameItem(id) => self.open_inline_editor(window, cx, id),
        }
    }

    /// Handles keystroke events from inner components.
    fn handle_keystroke(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        event: &KeystrokeEvent,
    ) {
        if let Some(action) = &event.action
            && event
                .context_stack
                .iter()
                .any(|context| context.contains(CONNECTION_DIALOG))
        {
            match action.name() {
                "ui::SelectDown" | "ui::SelectUp" => {
                    self.open_editor(window, cx, self.selected_connection(cx));
                }
                "ui::SelectLeft" => {
                    if let Some(id) = self.selected_folder(cx) {
                        self.expanded.remove(&id);
                    }
                }
                "ui::SelectRight" => {
                    if let Some(id) = self.selected_folder(cx) {
                        self.expanded.insert(id);
                    }
                }
                _ => {}
            }

            cx.notify();
        }
    }

    /// Adds a new default connection.
    fn add_connection(&self, cx: &mut Context<Self>, folder: Option<ConnectionFolderId>) {
        let mut connection = Connection::default();
        connection.set_folder(folder);

        ConnectionManager::global_mut(cx).add_connection(connection);
        self.load_tree(cx);
    }

    /// Adds a new default folder.
    fn add_folder(&self, cx: &mut Context<Self>, parent: Option<ConnectionFolderId>) {
        let mut folder = ConnectionFolder::default();
        folder.set_parent(parent);

        ConnectionManager::global_mut(cx).add_folder(folder);
        self.load_tree(cx);
    }

    /// Closes the connection open in the connection editor.
    fn close_editor(&self, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.close();
        })
    }

    /// Closes the item open in the inline editor.
    fn close_inline_editor(&mut self) {
        self.inline_editor_id = None;
    }

    /// Deletes the connection with the given [`ConnectionId`].
    fn delete_connection(&self, cx: &mut Context<Self>, id: &ConnectionId) {
        // Close the editor if we are deleting the connection that was being edited.
        if let Some(editor_id) = self.editor.read(cx).connection_id()
            && editor_id == *id
        {
            self.close_editor(cx);
        }

        ConnectionManager::global_mut(cx).remove_connection(id);
        self.load_tree(cx);
    }

    /// Deletes the folder with the given [`ConnectionFolderId`].
    fn delete_folder(&self, cx: &mut Context<Self>, id: &ConnectionFolderId) {
        let removed_connections = ConnectionManager::global_mut(cx).remove_folder(id);

        // Close the connection editor if the connection that was being edited was inside the deleted folder.
        if let Some(editor_id) = self.editor.read(cx).connection_id()
            && removed_connections.contains(&editor_id)
        {
            self.close_editor(cx);
        }

        self.load_tree(cx);
    }

    /// Duplicates the connection with the given [`ConnectionId`].
    fn duplicate_connection(&self, cx: &mut Context<Self>, id: &ConnectionId) {
        ConnectionManager::global_mut(cx).duplicate_connection(id);
        self.load_tree(cx);
    }

    /// Duplicates the folder with the given [`ConnectionFolderId`].
    fn duplicate_folder(&self, cx: &mut Context<Self>, id: &ConnectionFolderId) {
        ConnectionManager::global_mut(cx).duplicate_folder(id);
        self.load_tree(cx);
    }

    /// Returns whether the connection with the given id string is active within the connection editor.
    fn is_editing(&self, cx: &App, id: &SharedString) -> bool {
        if let Some(editor_id) = self.editor.read(cx).connection_id()
            && editor_id.to_string() == id.as_ref()
        {
            true
        } else {
            false
        }
    }

    /// Returns whether the connection editor is empty (i.e. does not have a connection open).
    fn is_editor_empty(&self, cx: &App) -> bool {
        self.editor.read(cx).variant.is_none()
    }

    /// Returns whether the folder with the given [`ConnectionFolderId`] is expanded.
    fn is_expanded(&self, id: &ConnectionFolderId) -> bool {
        self.expanded.contains(id)
    }

    /// Returns whether the item with the given id string is active within the inline editor.
    fn is_inline_editing(&self, id: &SharedString) -> bool {
        if let Some(editor_id) = &self.inline_editor_id
            && editor_id == id
        {
            true
        } else {
            false
        }
    }

    /// Moves the connection with the given [`ConnectionId`] to the given folder.
    fn move_connection(
        &mut self,
        cx: &mut Context<Self>,
        id: &ConnectionId,
        folder: Option<ConnectionFolderId>,
    ) {
        // Do not move the connection if it's already inside the given folder.
        if folder == ConnectionManager::global(cx).connection(id).folder() {
            return;
        }

        // Expand the folder the connection is being moved to.
        if let Some(folder) = folder {
            self.expanded.insert(folder);
        }

        ConnectionManager::global_mut(cx).move_connection(id, folder);
        self.load_tree(cx);
    }

    /// Moves the folder with the given [`ConnectionFolderId`] to the given parent folder.
    fn move_folder(
        &mut self,
        cx: &mut Context<Self>,
        id: &ConnectionFolderId,
        parent: Option<ConnectionFolderId>,
    ) {
        // Do not move the folder if we are trying to move it inside itself.
        if let Some(parent) = parent
            && (parent == *id || ConnectionManager::global(cx).folder_contains_folder(id, &parent))
        {
            return;
        }

        // Expand the parent the folder is being moved to.
        if let Some(parent) = parent {
            self.expanded.insert(parent);
        }

        ConnectionManager::global_mut(cx).move_folder(id, parent);
        self.load_tree(cx);
    }

    /// Emits an event to open the connection that is active within the connection editor.
    ///
    /// Opening the connection in this context means loading the connection and its information into the application.
    fn open_connection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_inline_editor(window, cx);
        self.save_editor(cx);

        // Do not open another connection if one is already opening.
        if self.opening {
            return;
        }

        if let Some(id) = self.editor.read(cx).connection_id() {
            let this = cx.entity();
            let event = Event::new(EventVariant::OpenConnection(id))
                .on_complete({
                    let this = this.clone();
                    move |window, cx| {
                        this.update(cx, |this, cx| {
                            this.reset(cx);
                            window.close_dialog(cx);
                        });
                    }
                })
                .on_error({
                    let this = this.clone();
                    move |error, window, cx| {
                        let error = error.to_owned();
                        window.open_alert_dialog(cx, move |dialog, _, cx| {
                            components::error_dialog(dialog, cx, error.clone())
                        });

                        this.update(cx, |this, _| {
                            this.opening = false;
                        });
                    }
                });

            self.opening = true;
            EventManager::emit(window, cx, event);
            cx.notify();
        }
    }

    /// Opens the connection editor with the given [`ConnectionId`], if any.
    ///
    /// Closes the connection editor if [`None`] is given.
    fn open_editor(&self, window: &mut Window, cx: &mut Context<Self>, id: Option<ConnectionId>) {
        self.editor.update(cx, |editor, cx| {
            if let Some(id) = id {
                editor.open(window, cx, id);
            } else {
                editor.close();
            }
        });
    }

    /// Opens the inline editor with the given id string.
    fn open_inline_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        id: impl Into<SharedString>,
    ) {
        let id = id.into();
        let manager = ConnectionManager::global(cx);
        let name = if let Some(connection) = manager.try_connection(&id) {
            connection.name()
        } else if let Some(folder) = manager.try_folder(&id) {
            folder.name()
        } else {
            return;
        };

        self.inline_editor_id = Some(id);
        self.inline_name_input.update(cx, |inline_name_input, cx| {
            inline_name_input.focus(window, cx);
            inline_name_input.set_value(name, window, cx);
        });
    }

    /// Resets the temporary state of the dialog.
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.opening = false;
        self.expanded.clear();
        self.close_editor(cx);
        self.load_tree(cx);
        self.tree.update(cx, |tree, cx| {
            tree.set_selected_item(None, cx);
        });
    }

    /// Saves the connection that is active within the connection editor.
    fn save_editor(&self, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);

        if let Some(id) = editor.connection_id()
            && let Some(options) = editor.options(cx)
        {
            let name = editor.name(cx);
            let manager = ConnectionManager::global_mut(cx);
            let connection = manager.connection_mut(&id);
            connection.set_name(name);
            connection.set_options(options);
            manager.save();

            // Reload the tree with the newly saved connection.
            self.load_tree(cx);
        }
    }

    /// Saves the item that is active within the inline editor.
    fn save_inline_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = &self.inline_editor_id else {
            return;
        };

        let name = self.inline_name_input.read(cx).value();

        // If the item that is active within the inline editor is also active within the connection editor,
        // we want to synchronize the name within the connection editor with the name from the inline editor.
        if self.is_editing(cx, id) {
            self.editor.update(cx, |editor, cx| {
                editor.set_name(window, cx, name.clone());
            });
        }

        let manager = ConnectionManager::global_mut(cx);
        if let Some(connection) = manager.try_connection_mut(id) {
            connection.set_name(name);
        } else if let Some(folder) = manager.try_folder_mut(id) {
            folder.set_name(name);
        }

        manager.save();
        self.close_inline_editor();
        self.load_tree(cx);
    }

    /// Returns the selected connection, if any.
    fn selected_connection(&self, cx: &App) -> Option<ConnectionId> {
        if let Some(item) = self.tree.read(cx).selected_item()
            && let Some(connection) = ConnectionManager::global(cx).try_connection(&item.id)
        {
            Some(connection.id())
        } else {
            None
        }
    }

    /// Returns the selected folder, if any.
    fn selected_folder(&self, cx: &App) -> Option<ConnectionFolderId> {
        if let Some(item) = self.tree.read(cx).selected_item()
            && let Some(parent) = ConnectionManager::global(cx).try_folder(&item.id)
        {
            Some(parent.id())
        } else {
            None
        }
    }

    /// Returns the parent folder of the selected item, if any.
    /// - When a connection is selected, this will return the [`ConnectionFolderId`] of its parent, if one exists.
    /// - When a folder is selected, this will return the [`ConnectionFolderId`] of that folder.
    fn selected_parent(&self, cx: &App) -> Option<ConnectionFolderId> {
        if let Some(id) = self.selected_connection(cx) {
            ConnectionManager::global(cx).connection(&id).folder()
        } else {
            self.selected_folder(cx)
        }
    }

    /// Toggles the expanded state of the folder with the given [`ConnectionFolderId`].
    fn toggle_expand(&mut self, cx: &mut Context<Self>, id: ConnectionFolderId) {
        if self.is_expanded(&id) {
            self.expanded.remove(&id);
        } else {
            self.expanded.insert(id);
        }

        self.load_tree(cx);
    }

    fn load_tree(&self, cx: &mut Context<Self>) {
        let filter = self.search_input.read(cx).value();
        let items = self.build_tree_items(ConnectionManager::global(cx), &filter, None);
        self.tree.update(cx, |tree, cx| {
            let selected = tree.selected_item().cloned();
            tree.set_items(items, cx);
            tree.set_selected_item(selected.as_ref(), cx);
        });
    }

    fn build_tree_items(
        &self,
        manager: &ConnectionManager,
        filter: &SharedString,
        folder_id: Option<ConnectionFolderId>,
    ) -> Vec<TreeItem> {
        let mut items = Vec::new();

        // Build any nested folders inside the folder.
        if let Some(children_folders) = manager.folders_for_parent(&folder_id) {
            let mut folder_items = Vec::new();

            for id in children_folders {
                let folder = manager.folder(id);
                let children = self.build_tree_items(manager, filter, Some(*id));

                // If there's an active filter, we only want to show folders that contain connections matching the filter.
                if filter.is_empty() || !children.is_empty() {
                    let item = TreeItem::new(id.to_string(), folder.name())
                        .expanded(self.is_expanded(id))
                        .children(children);

                    folder_items.push(item);
                }
            }

            folder_items.sort_unstable_by(|a, b| a.label.cmp(&b.label));
            items.extend(folder_items);
        }

        // Build any connections inside the folder.
        if let Some(children_connections) = manager.connections_for_folder(&folder_id) {
            let mut connection_items = Vec::new();

            for id in children_connections {
                let connection = manager.connection(id);

                // Only include connections that match the filter.
                if filter.is_empty() || connection.name().to_lowercase().contains(filter.as_str()) {
                    let item = TreeItem::new(id.to_string(), connection.name());
                    connection_items.push(item);
                }
            }

            connection_items.sort_unstable_by(|a, b| a.label.cmp(&b.label));
            items.extend(connection_items);
        }

        items
    }
}

/// The state used with a [`ConnectionEditor`].
struct ConnectionEditorState {
    /// The state for the name input.
    name_input: Entity<InputState>,

    /// The state for the connection type select.
    type_select: Entity<SelectState<Vec<SharedString>>>,

    /// The variant of the editor.
    ///
    /// This will be [`None`] if we do not have a connection selected to edit.
    variant: Option<ConnectionEditorVariant>,
}

impl ConnectionEditorState {
    /// Creates a new [`ConnectionEditorState`].
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

    /// Closes the connection open inside the editor.
    fn close(&mut self) {
        self.variant = None;
    }

    /// Returns the [`ConnectionId`] of the connection open in the editor, if any.
    fn connection_id(&self) -> Option<ConnectionId> {
        self.variant
            .as_ref()
            .map(|ConnectionEditorVariant::MySql(state)| state.id)
    }

    /// Returns the name entered inside the editor.
    fn name(&self, cx: &App) -> SharedString {
        self.name_input.read(cx).value()
    }

    /// Opens the connection with the given [`ConnectionId`] inside the editor.
    fn open(&mut self, window: &mut Window, cx: &mut App, id: ConnectionId) {
        let connection = ConnectionManager::global(cx).connection(&id).clone();
        self.set_name(window, cx, connection.name());

        self.type_select.update(cx, |type_select, cx| {
            type_select.set_selected_value(&SharedString::from(""), window, cx);
        });

        self.variant = Some(match connection.options() {
            ConnectionOptions::MySql(options) => {
                let state = MySqlEditorState::new(window, cx, &connection, options);
                ConnectionEditorVariant::MySql(state)
            }
        });
    }

    /// Returns the [`ConnectionOptions`] generated from the editor's entry fields, if any.
    fn options(&self, cx: &App) -> Option<ConnectionOptions> {
        self.variant
            .as_ref()
            .map(|ConnectionEditorVariant::MySql(state)| state.options(cx))
    }

    /// Sets the name inside the editor.
    fn set_name(&self, window: &mut Window, cx: &mut App, name: SharedString) {
        self.name_input.update(cx, |name_input, cx| {
            name_input.set_value(name, window, cx);
        });
    }
}

enum ConnectionEditorVariant {
    MySql(MySqlEditorState),
}

#[derive(IntoElement)]
struct ConnectionEditor {
    /// The state for the editor.
    state: Entity<ConnectionEditorState>,
}

impl ConnectionEditor {
    /// Creates a new [`ConnectionEditor`].
    fn new(state: &Entity<ConnectionEditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for ConnectionEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        v_flex().gap_1().p_3().w_full().map(|this| {
            if let Some(variant) = &state.variant {
                this.child(labeled("Name", Input::new(&state.name_input)))
                    //.child(labeled("Connection Type", Select::new(&state.type_select)))
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

/// The state used when editing a MySQL connection inside a [`ConnectionEditor`].
struct MySqlEditorState {
    /// The id of the connection being edited.
    id: ConnectionId,

    /// The state for the host input.
    host_input: Entity<InputState>,

    /// The state for the port input.
    port_input: Entity<InputState>,

    /// The state for the user input.
    user_input: Entity<InputState>,

    /// The state for the password input.
    password_input: Entity<InputState>,
}

impl MySqlEditorState {
    /// Creates a new [`MySqlEditorState`].
    fn new(
        window: &mut Window,
        cx: &mut App,
        connection: &Connection,
        options: MySqlOptions,
    ) -> Self {
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
            id: connection.id(),
            host_input,
            port_input,
            user_input,
            password_input,
        }
    }

    /// Returns the [`ConnectionOptions`] generated from the editor's entry fields.
    fn options(&self, cx: &App) -> ConnectionOptions {
        let options = MySqlOptions {
            host: self.host_input.read(cx).value().to_string(),
            port: self
                .port_input
                .read(cx)
                .value()
                .parse::<u16>()
                .unwrap_or(migris::shared::DEFAULT_MYSQL_PORT),
            user: self.user_input.read(cx).value().to_string(),
            password: self.password_input.read(cx).value().to_string(),
        };

        ConnectionOptions::MySql(options)
    }
}
