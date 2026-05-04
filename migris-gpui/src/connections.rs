use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::anyhow;
use directories::BaseDirs;
use gpui::{App, Global, SharedString};
use migris::connection::ConnectionOptions;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{secrets, shared};

pub struct ConnectionManager {
    /// The config containing the connections and folders.
    config: ConnectionConfig,

    /// Tracks the locations of connections within the full list by [`ConnectionId`].
    connection_map: HashMap<ConnectionId, usize>,

    /// Tracks the connections within each folder.
    connections_by_folder: HashMap<Option<ConnectionFolderId>, Vec<ConnectionId>>,

    /// Tracks the locations of folders within the full list by [`ConnectionFolderId`].
    folder_map: HashMap<ConnectionFolderId, usize>,

    /// Tracks the folders within each parent folder.
    folders_by_parent: HashMap<Option<ConnectionFolderId>, Vec<ConnectionFolderId>>,
}

impl Global for ConnectionManager {}

impl ConnectionManager {
    /// Loads the config from the config file.
    pub fn load() -> Self {
        let config = Self::try_load().unwrap_or_else(|_| ConnectionConfig::default());
        let mut manager = Self {
            config,
            connection_map: HashMap::new(),
            connections_by_folder: HashMap::new(),
            folder_map: HashMap::new(),
            folders_by_parent: HashMap::new(),
        };

        manager.load_maps();
        manager
    }

    /// Saves the config to the config file.
    pub fn save(&mut self) {
        // TODO: log errors with saving
        _ = self.try_save();
    }

    /// Returns a reference to the global [`ConnectionManager`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns a mutable reference to the global [`ConnectionManager`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Adds a new connection to the config.
    pub fn add_connection(&mut self, connection: Connection) {
        self.config.connections.push(connection);
        self.load_maps();
        self.save();
    }

    /// Adds a new folder to the config.
    pub fn add_folder(&mut self, folder: ConnectionFolder) {
        self.config.folders.push(folder);
        self.load_maps();
        self.save();
    }

    /// Returns a reference to the connection matching the given [`ConnectionId`].
    pub fn connection(&self, id: &ConnectionId) -> &Connection {
        let idx = self.connection_map[id];
        &self.config.connections[idx]
    }

    /// Returns a mutable reference to the connection matching the given [`ConnectionId`].
    pub fn connection_mut(&mut self, id: &ConnectionId) -> &mut Connection {
        let idx = self.connection_map[id];
        &mut self.config.connections[idx]
    }

    /// Returns the connections within the given folder.
    pub fn connections_for_folder(
        &self,
        folder: &Option<ConnectionFolderId>,
    ) -> Option<&Vec<ConnectionId>> {
        self.connections_by_folder.get(folder)
    }

    /// Returns a reference to the folder matching the given [`ConnectionFolderId`].
    pub fn folder(&self, id: &ConnectionFolderId) -> &ConnectionFolder {
        let idx = self.folder_map[id];
        &self.config.folders[idx]
    }

    /// Returns the folders within the given parent folder.
    pub fn folders_for_parent(
        &self,
        parent: &Option<ConnectionFolderId>,
    ) -> Option<&Vec<ConnectionFolderId>> {
        self.folders_by_parent.get(parent)
    }

    /// Removes the connection with the given [`ConnectionId`] from the config.
    pub fn remove_connection(&mut self, id: &ConnectionId) {
        let idx = self.connection_map[id];
        self.config.connections.swap_remove(idx);
        self.load_maps();
        self.save();
    }

    /// Removes the folder with the given [`ConnectionFolderId`] from the config.
    ///
    /// Returns the set of connections that were removed for future processing if needed.
    pub fn remove_folder(&mut self, id: &ConnectionFolderId) -> HashSet<ConnectionId> {
        fn remove_inner(
            manager: &ConnectionManager,
            id: ConnectionFolderId,
        ) -> (HashSet<ConnectionId>, HashSet<ConnectionFolderId>) {
            let mut removed_connections = HashSet::new();
            let mut removed_folders = HashSet::from([id]);

            if let Some(connections) = manager.connections_for_folder(&Some(id)) {
                removed_connections.extend(connections);
            }

            if let Some(folders) = manager.folders_for_parent(&Some(id)) {
                for id in folders {
                    let (inner_removed_connections, inner_removed_folders) =
                        remove_inner(manager, *id);
                    removed_connections.extend(inner_removed_connections);
                    removed_folders.extend(inner_removed_folders);
                }
            }

            (removed_connections, removed_folders)
        }

        let (removed_connections, removed_folders) = remove_inner(self, *id);
        self.config
            .connections
            .retain(|connection| !removed_connections.contains(&connection.id));
        self.config
            .folders
            .retain(|folder| !removed_folders.contains(&folder.id));

        self.load_maps();
        self.save();
        removed_connections
    }

    /// Returns a reference to the connection matching the given id string, if one is found.
    pub fn try_connection(&self, id: &SharedString) -> Option<&Connection> {
        if let Ok(uuid) = Uuid::parse_str(id)
            && let Some(idx) = self.connection_map.get(&ConnectionId(uuid))
            && let Some(connection) = self.config.connections.get(*idx)
        {
            Some(connection)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the connection matching the given id string, if one is found.
    pub fn try_connection_mut(&mut self, id: &SharedString) -> Option<&mut Connection> {
        if let Ok(uuid) = Uuid::parse_str(id)
            && let Some(idx) = self.connection_map.get(&ConnectionId(uuid))
            && let Some(connection) = self.config.connections.get_mut(*idx)
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
            && let Some(folder) = self.config.folders.get(*idx)
        {
            Some(folder)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the folder matching the given id string, if one is found.
    pub fn try_folder_mut(&mut self, id: &SharedString) -> Option<&mut ConnectionFolder> {
        if let Ok(uuid) = Uuid::parse_str(id)
            && let Some(idx) = self.folder_map.get(&ConnectionFolderId(uuid))
            && let Some(folder) = self.config.folders.get_mut(*idx)
        {
            Some(folder)
        } else {
            None
        }
    }

    fn load_maps(&mut self) {
        self.connection_map.clear();
        self.connections_by_folder.clear();
        self.folder_map.clear();
        self.folders_by_parent.clear();

        for (idx, connection) in self.config.connections.iter().enumerate() {
            self.connection_map.insert(connection.id, idx);
            self.connections_by_folder
                .entry(connection.folder)
                .or_default()
                .push(connection.id);
        }

        for (idx, folder) in self.config.folders.iter().enumerate() {
            self.folder_map.insert(folder.id, idx);
            self.folders_by_parent
                .entry(folder.parent)
                .or_default()
                .push(folder.id);
        }
    }

    fn connections_path() -> Result<PathBuf, anyhow::Error> {
        let Some(dirs) = BaseDirs::new() else {
            return Err(anyhow!("Failed to retrieve directories"));
        };

        Ok(dirs
            .config_dir()
            .join(shared::APPLICATION_NAME)
            .join("connections.json"))
    }

    fn try_load() -> Result<ConnectionConfig, anyhow::Error> {
        let path = Self::connections_path()?;
        let reader = BufReader::new(File::open(path)?);
        Ok(serde_json::from_reader(reader)?)
    }

    fn try_save(&self) -> Result<(), anyhow::Error> {
        let path = Self::connections_path()?;
        let writer = BufWriter::new(File::create(path)?);
        serde_json::to_writer_pretty(writer, &self.config)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ConnectionConfig {
    connections: Vec<Connection>,
    folders: Vec<ConnectionFolder>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Connection {
    /// The id of the connection.
    id: ConnectionId,

    /// The optional id of the folder containing the connection.
    folder: Option<ConnectionFolderId>,

    /// The name of the connection.
    name: String,

    /// The options for the connection.
    options: ConnectionOptions,
}

impl Connection {
    /// Creates a new [`Connection`].
    pub fn new(name: impl Into<String>, options: ConnectionOptions) -> Self {
        Self {
            id: ConnectionId(Uuid::now_v7()),
            folder: None,
            name: name.into(),
            options,
        }
    }

    /// Returns the [`ConnectionId`] of the connection.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Returns the [`ConnectionFolderId`] of the folder containing the connection, if any.
    pub fn folder(&self) -> Option<ConnectionFolderId> {
        self.folder
    }

    /// Returns the name of the connection.
    pub fn name(&self) -> SharedString {
        SharedString::from(&self.name)
    }

    /// Returns the options for the connection.
    pub fn options(&self) -> ConnectionOptions {
        let mut options = self.options.clone();
        match &mut options {
            ConnectionOptions::MySql(options) => {
                options.password = self.password();
            }
        }

        options
    }

    /// Sets the folder of the connection.
    pub fn set_folder(&mut self, id: ConnectionFolderId) {
        self.folder = Some(id);
    }

    /// Sets the name of the connection.
    pub fn set_name(&mut self, name: SharedString) {
        self.name = name.to_string();
    }

    /// Sets the options for the connection.
    pub fn set_options(&mut self, options: ConnectionOptions) {
        self.options = options;
        self.set_password();
    }

    /// Returns the password for the connection.
    ///
    /// This will attempt to retrieve the password from the system key storage,
    /// and fallback to the originally stored password if that fails.
    fn password(&self) -> String {
        match &self.options {
            ConnectionOptions::MySql(options) => {
                secrets::get_secret(&options.password).unwrap_or(options.password.clone())
            }
        }
    }

    /// Sets the password for the connection.
    ///
    /// This will attempt to store the password in the system key storage,
    /// and fallback to keeping the plain password if that fails.
    fn set_password(&mut self) {
        match &mut self.options {
            ConnectionOptions::MySql(options) => {
                let secret = format!("{}:password", self.id);
                if secrets::set_secret(&secret, &options.password).is_ok() {
                    options.password = secret;
                }
            }
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new("New connection", ConnectionOptions::default())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionFolderId(Uuid);

impl Display for ConnectionFolderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionFolder {
    /// The id of the folder.
    id: ConnectionFolderId,

    /// The name of the folder.
    name: String,

    /// The optional id of the parent folder containing the folder.
    parent: Option<ConnectionFolderId>,
}

impl ConnectionFolder {
    /// Creates a new [`ConnectionFolder`].
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ConnectionFolderId(Uuid::now_v7()),
            parent: None,
            name: name.into(),
        }
    }

    /// Returns the [`ConnectionFolderId`] of the folder.
    pub fn id(&self) -> ConnectionFolderId {
        self.id
    }

    /// Returns the name of the folder.
    pub fn name(&self) -> SharedString {
        SharedString::from(&self.name)
    }

    /// Sets the name of the folder.
    pub fn set_name(&mut self, name: SharedString) {
        self.name = name.to_string();
    }

    /// Sets the parent of the folder.
    pub fn set_parent(&mut self, id: ConnectionFolderId) {
        self.parent = Some(id);
    }
}

impl Default for ConnectionFolder {
    fn default() -> Self {
        Self::new("New folder")
    }
}
