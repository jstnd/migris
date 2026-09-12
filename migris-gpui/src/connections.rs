use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    sync::Arc,
};

use anyhow::Result;
use gpui_kit::{App, Global, SharedString, Task};
use migris::drivers::ConnectionKind;
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{database::Database, secrets};

pub struct ConnectionManager {
    /// The application's database.
    database: Arc<Database>,

    /// The saved connections.
    connections: Vec<Connection>,

    /// Tracks the locations of connections within the full list by [`ConnectionId`].
    connection_map: HashMap<ConnectionId, usize>,

    /// Tracks the connections within each folder.
    connections_by_folder: HashMap<Option<ConnectionFolderId>, Vec<ConnectionId>>,

    /// The saved connection folders.
    folders: Vec<ConnectionFolder>,

    /// Tracks the locations of folders within the full list by [`ConnectionFolderId`].
    folder_map: HashMap<ConnectionFolderId, usize>,

    /// Tracks the folders within each parent folder.
    folders_by_parent: HashMap<Option<ConnectionFolderId>, Vec<ConnectionFolderId>>,
}

impl Global for ConnectionManager {}

impl ConnectionManager {
    /// Creates a new [`ConnectionManager`].
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            connections: Vec::new(),
            connection_map: HashMap::new(),
            connections_by_folder: HashMap::new(),
            folders: Vec::new(),
            folder_map: HashMap::new(),
            folders_by_parent: HashMap::new(),
        }
    }

    /// Initializes the in-memory connections and folders from the database.
    pub fn init(&self, cx: &App) -> Task<Result<()>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            let connections = database.connections().await?;
            let folders = database.connection_folders().await?;
            cx.update_global(|this: &mut Self, _| {
                this.connections = connections;
                this.folders = folders;
                this.load_maps();
            });

            Ok(())
        })
    }

    /// Loads the in-memory connection and folder mappings.
    ///
    /// These mappings are used for more efficient access to the saved connections and folders.
    fn load_maps(&mut self) {
        self.connection_map.clear();
        self.connections_by_folder.clear();
        self.folder_map.clear();
        self.folders_by_parent.clear();

        for (idx, connection) in self.connections.iter().enumerate() {
            self.connection_map.insert(connection.id, idx);
            self.connections_by_folder
                .entry(connection.folder_id)
                .or_default()
                .push(connection.id);
        }

        for (idx, folder) in self.folders.iter().enumerate() {
            self.folder_map.insert(folder.id, idx);
            self.folders_by_parent
                .entry(folder.folder_id)
                .or_default()
                .push(folder.id);
        }
    }

    /// Returns a reference to the global [`ConnectionManager`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Adds a new connection to the saved connections.
    pub fn add_connection(&self, cx: &App, connection: Connection) -> Task<Result<()>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.insert_connection(&connection).await?;
            cx.update_global(|this: &mut Self, _| {
                let next_idx = this.connections.len();
                this.connection_map.insert(connection.id, next_idx);
                this.connections_by_folder
                    .entry(connection.folder_id)
                    .or_default()
                    .push(connection.id);
                this.connections.push(connection);
            });

            Ok(())
        })
    }

    /// Adds a new folder to the saved folders.
    pub fn add_folder(&self, cx: &App, folder: ConnectionFolder) -> Task<Result<()>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.insert_connection_folder(&folder).await?;
            cx.update_global(|this: &mut Self, _| {
                let next_idx = this.folders.len();
                this.folder_map.insert(folder.id, next_idx);
                this.folders_by_parent
                    .entry(folder.folder_id)
                    .or_default()
                    .push(folder.id);
                this.folders.push(folder);
            });

            Ok(())
        })
    }

    /// Returns a reference to the connection matching the given [`ConnectionId`].
    pub fn connection(&self, id: &ConnectionId) -> &Connection {
        let idx = self.connection_map[id];
        &self.connections[idx]
    }

    /// Returns a mutable reference to the connection matching the given [`ConnectionId`].
    fn connection_mut(&mut self, id: &ConnectionId) -> &mut Connection {
        let idx = self.connection_map[id];
        &mut self.connections[idx]
    }

    /// Returns the connections within the given folder.
    pub fn connections_for_folder(
        &self,
        folder: &Option<ConnectionFolderId>,
    ) -> Option<&Vec<ConnectionId>> {
        self.connections_by_folder.get(folder)
    }

    /// Deletes the connection with the given [`ConnectionId`].
    pub fn delete_connection(&self, cx: &App, id: ConnectionId) -> Task<Result<()>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.delete_connection(&id).await?;
            cx.update_global(|this: &mut Self, _| {
                let idx = this.connection_map[&id];
                let connection = &this.connections[idx];

                // Clean up system key storage.
                connection.delete_password();

                this.connections.swap_remove(idx);
                this.load_maps();
            });

            Ok(())
        })
    }

    /// Deletes the folder with the given [`ConnectionFolderId`].
    ///
    /// Returns the set of connections that were deleted for future processing if needed.
    pub fn delete_folder(
        &self,
        cx: &App,
        id: ConnectionFolderId,
    ) -> Task<Result<HashSet<ConnectionId>>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.delete_connection_folder(&id).await?;
            let deleted_connections = cx.update_global(|this: &mut Self, _| {
                let mut deleted_connections = HashSet::new();
                let mut queue = VecDeque::from([id]);

                while let Some(folder_id) = queue.pop_front() {
                    if let Some(connections) = this.connections_for_folder(&Some(folder_id)) {
                        deleted_connections.extend(connections);
                    }

                    if let Some(folders) = this.folders_for_parent(&Some(folder_id)) {
                        queue.extend(folders);
                    }
                }

                // Clean up system key storage.
                for connection_id in deleted_connections.iter() {
                    let connection = this.connection(connection_id);
                    connection.delete_password();
                }

                deleted_connections
            });

            cx.read_global(|this: &Self, cx| this.init(cx)).await?;
            Ok(deleted_connections)
        })
    }

    /// Duplicates the connection with the given [`ConnectionId`].
    pub fn duplicate_connection(&self, cx: &App, id: &ConnectionId) -> Task<Result<()>> {
        let connection = self.connection(id);
        self.add_connection(
            cx,
            connection.duplicate_with_name(format!("{} - Copy", connection.name)),
        )
    }

    /// Returns a reference to the folder matching the given [`ConnectionFolderId`].
    pub fn folder(&self, id: &ConnectionFolderId) -> &ConnectionFolder {
        let idx = self.folder_map[id];
        &self.folders[idx]
    }

    /// Returns a mutable reference to the folder matching the given [`ConnectionFolderId`].
    fn folder_mut(&mut self, id: &ConnectionFolderId) -> &mut ConnectionFolder {
        let idx = self.folder_map[id];
        &mut self.folders[idx]
    }

    /// Returns whether the folder with the given [`ConnectionFolderId`] contains the folder with the other [`ConnectionFolderId`].
    pub fn folder_contains_folder(
        &self,
        id: &ConnectionFolderId,
        other: &ConnectionFolderId,
    ) -> bool {
        if let Some(children_folders) = self.folders_for_parent(&Some(*id)) {
            children_folders.contains(other)
                || children_folders
                    .iter()
                    .any(|id| self.folder_contains_folder(id, other))
        } else {
            false
        }
    }

    /// Returns the folders within the given parent folder.
    pub fn folders_for_parent(
        &self,
        parent: &Option<ConnectionFolderId>,
    ) -> Option<&Vec<ConnectionFolderId>> {
        self.folders_by_parent.get(parent)
    }

    /// Moves the connection with the given [`ConnectionId`] to the given folder.
    pub fn move_connection(
        &self,
        cx: &App,
        id: ConnectionId,
        folder_id: Option<ConnectionFolderId>,
    ) -> Task<Result<()>> {
        let mut connection = self.connection(&id).clone();
        let prev_folder_id = connection.folder_id;
        connection.folder_id = folder_id;

        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.update_connection(&connection).await?;
            cx.update_global(|this: &mut Self, _| {
                this.connection_mut(&id).folder_id = folder_id;

                // Remove connection from previous folder in mapping.
                this.connections_by_folder
                    .entry(prev_folder_id)
                    .or_default()
                    .retain(|inner_id| *inner_id != id);

                // Add connection to new folder in mapping.
                this.connections_by_folder
                    .entry(folder_id)
                    .or_default()
                    .push(id);
            });

            Ok(())
        })
    }

    /// Moves the folder with the given [`ConnectionFolderId`] to the given parent folder.
    pub fn move_folder(
        &self,
        cx: &App,
        id: ConnectionFolderId,
        folder_id: Option<ConnectionFolderId>,
    ) -> Task<Result<()>> {
        let mut folder = self.folder(&id).clone();
        let prev_folder_id = folder.folder_id;
        folder.folder_id = folder_id;

        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.update_connection_folder(&folder).await?;
            cx.update_global(|this: &mut Self, _| {
                this.folder_mut(&id).folder_id = folder_id;

                // Remove folder from previous parent in mapping.
                this.folders_by_parent
                    .entry(prev_folder_id)
                    .or_default()
                    .retain(|inner_id| *inner_id != id);

                // Add folder to new parent in mapping.
                this.folders_by_parent
                    .entry(folder_id)
                    .or_default()
                    .push(id);
            });

            Ok(())
        })
    }

    /// Returns a reference to the connection matching the given id string, if one is found.
    pub fn try_connection(&self, id: &SharedString) -> Option<&Connection> {
        if let Ok(uuid) = Uuid::parse_str(id)
            && let Some(idx) = self.connection_map.get(&ConnectionId(uuid))
            && let Some(connection) = self.connections.get(*idx)
        {
            Some(connection)
        } else {
            None
        }
    }

    /// Returns a reference to the folder matching the given id string, if one is found.
    pub fn try_folder(&self, id: &SharedString) -> Option<&ConnectionFolder> {
        if let Ok(uuid) = Uuid::parse_str(id)
            && let Some(idx) = self.folder_map.get(&ConnectionFolderId(uuid))
            && let Some(folder) = self.folders.get(*idx)
        {
            Some(folder)
        } else {
            None
        }
    }

    /// Updates the given [`Connection`].
    ///
    /// This will persist the connection's data in the application's database and in-memory.
    pub fn update_connection(&self, cx: &App, connection: Connection) -> Task<Result<()>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            let mut connection = connection;
            connection.set_password();
            database.update_connection(&connection).await?;
            cx.update_global(|this: &mut Self, _| {
                // Update in-memory connection to passed instance.
                let idx = this.connection_map[&connection.id];
                this.connections[idx] = connection;
                this.load_maps();
            });

            Ok(())
        })
    }

    /// Updates the given [`ConnectionFolder`].
    ///
    /// This will persist the folder's data in the application's database and in-memory.
    pub fn update_folder(&self, cx: &App, folder: ConnectionFolder) -> Task<Result<()>> {
        let database = self.database.clone();
        cx.spawn(async move |cx| {
            database.update_connection_folder(&folder).await?;
            cx.update_global(|this: &mut Self, _| {
                // Update in-memory folder to passed instance.
                let idx = this.folder_map[&folder.id];
                this.folders[idx] = folder;
                this.load_maps();
            });

            Ok(())
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    /// Creates a new [`ConnectionId`].
    fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Connection {
    /// The id of the connection.
    pub id: ConnectionId,

    /// The optional [`ConnectionFolderId`] of the folder containing the connection.
    pub folder_id: Option<ConnectionFolderId>,

    /// The name of the connection.
    pub name: String,

    /// The kind of the connection.
    pub kind: ConnectionKind,

    /// The host for the connection.
    ///
    /// NOTE: For SQLite connections, this will contain the path to the database file.
    pub host: String,

    /// The port for the connection.
    pub port: u16,

    /// The username for the connection.
    pub username: String,

    /// The password for the connection.
    pub password: String,

    /// The date and time the connection was created in UTC.
    pub created_at: DateTime<Utc>,

    /// The date and time the connection was last opened.
    pub last_connected: Option<DateTime<Utc>>,
}

impl Connection {
    /// Creates a new [`Connection`].
    pub fn new(name: impl Into<String>, kind: ConnectionKind) -> Self {
        Self {
            id: ConnectionId::new(),
            folder_id: None,
            name: name.into(),
            kind,
            host: String::new(),
            port: kind.default_port(),
            username: String::new(),
            password: String::new(),
            created_at: Utc::now(),
            last_connected: None,
        }
    }

    /// Returns the string that should be used to access the connection's database.
    pub fn connection_string(&self) -> String {
        match self.kind {
            ConnectionKind::MySql => format!(
                "mysql://{}:{}@{}:{}",
                self.username,
                self.password(),
                self.host,
                self.port
            ),
            ConnectionKind::Sqlite => self.host.clone(),
        }
    }

    /// Duplicates the connection, using the given name as the new connection's name.
    pub fn duplicate_with_name(&self, name: impl Into<String>) -> Self {
        let mut connection = self.clone();
        connection.id = ConnectionId::new();
        connection.name = name.into();
        connection.created_at = Utc::now();
        connection.last_connected = None;

        // Retrieve password from the connection being
        // duplicated and save it with the new connection.
        connection.password = connection.password();
        connection.set_password();
        connection
    }

    /// Deletes the password for the connection.
    ///
    /// This will delete the password from the system key storage.
    fn delete_password(&self) {
        _ = secrets::delete_secret(&self.password);
    }

    /// Returns the password for the connection.
    ///
    /// This will attempt to retrieve the password from the system key storage,
    /// and fallback to the originally stored password if that fails.
    pub fn password(&self) -> String {
        secrets::get_secret(&self.password).unwrap_or(self.password.clone())
    }

    /// Sets the password for the connection.
    ///
    /// This will attempt to store the password in the system key storage,
    /// and fallback to keeping the plain password if that fails.
    fn set_password(&mut self) {
        let secret = format!("{}:password", self.id);
        if secrets::set_secret(&secret, &self.password).is_ok() {
            self.password = secret;
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new("New connection", ConnectionKind::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct ConnectionFolderId(Uuid);

impl ConnectionFolderId {
    /// Creates a new [`ConnectionFolderId`].
    fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for ConnectionFolderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConnectionFolder {
    /// The id of the folder.
    pub id: ConnectionFolderId,

    /// The optional [`ConnectionFolderId`] of the folder containing this folder.
    pub folder_id: Option<ConnectionFolderId>,

    /// The name of the folder.
    pub name: String,
}

impl ConnectionFolder {
    /// Creates a new [`ConnectionFolder`].
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ConnectionFolderId::new(),
            folder_id: None,
            name: name.into(),
        }
    }
}

impl Default for ConnectionFolder {
    fn default() -> Self {
        Self::new("New folder")
    }
}
