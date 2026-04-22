use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::anyhow;
use directories::BaseDirs;
use gpui::{App, Global, SharedString};
use migris::connection::ConnectOptions;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared;

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct ConnectionConfig {
    connections: Vec<Connection>,
    folders: Vec<ConnectionFolder>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Connection {
    /// The id of the connection.
    id: ConnectionId,

    /// The optional id of the folder containing the connection.
    folder: Option<ConnectionFolderId>,

    /// The name of the connection.
    name: String,
    //options: ConnectOptions,
}

impl Connection {
    /// Returns the id of the connection.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Returns the id of the folder containing the connection, if any.
    pub fn folder(&self) -> Option<ConnectionFolderId> {
        self.folder
    }

    /// Returns the name of the connection.
    pub fn name(&self) -> SharedString {
        SharedString::from(&self.name)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ConnectionFolder {
    id: ConnectionFolderId,
    parent: Option<ConnectionFolderId>,
    name: String,
}

impl ConnectionFolder {
    /// Returns the id of the folder.
    pub fn id(&self) -> ConnectionFolderId {
        self.id
    }

    /// Returns the id of the parent folder, if any.
    pub fn parent(&self) -> Option<ConnectionFolderId> {
        self.parent
    }

    /// Returns the name of the folder.
    pub fn name(&self) -> SharedString {
        SharedString::from(&self.name)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionFolderId(Uuid);

impl Display for ConnectionFolderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct ConnectionManager {
    ///
    config: ConnectionConfig,

    ///
    saved_config: ConnectionConfig,

    ///
    connection_map: HashMap<ConnectionId, usize>,

    ///
    folder_map: HashMap<ConnectionFolderId, usize>,
}

impl Global for ConnectionManager {}

impl ConnectionManager {
    ///
    pub fn load() -> Self {
        let config = Self::try_load().unwrap_or_else(|_| ConnectionConfig::default());
        let saved_config = config.clone();
        let mut manager = Self {
            config,
            saved_config,
            connection_map: HashMap::new(),
            folder_map: HashMap::new(),
        };

        manager.load_maps();
        manager
    }

    ///
    pub fn revert(&mut self) {
        self.config = self.saved_config.clone();
    }

    ///
    pub fn save(&mut self) {
        // TODO: log errors with saving
        self.saved_config = self.config.clone();
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

    ///
    pub fn connections(&self) -> &Vec<Connection> {
        &self.config.connections
    }

    ///
    pub fn connection(&self, id: &ConnectionId) -> &Connection {
        let idx = self.connection_map[id];
        &self.config.connections[idx]
    }

    ///
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

    ///
    pub fn folders(&self) -> &Vec<ConnectionFolder> {
        &self.config.folders
    }

    ///
    pub fn folder(&self, id: &ConnectionFolderId) -> &ConnectionFolder {
        let idx = self.folder_map[id];
        &self.config.folders[idx]
    }

    ///
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

    ///
    fn load_maps(&mut self) {
        self.connection_map.clear();
        self.folder_map.clear();

        for (idx, connection) in self.config.connections.iter().enumerate() {
            self.connection_map.insert(connection.id, idx);
        }

        for (idx, folder) in self.config.folders.iter().enumerate() {
            self.folder_map.insert(folder.id, idx);
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
