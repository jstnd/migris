use std::{collections::HashMap, sync::Arc};

use gpui_kit::{App, Hsla, SharedString, component::Colorize};
use migris::{Entity, drivers::Driver};

use crate::connections::{Connection, ConnectionId, ConnectionManager};

pub struct OpenConnection {
    /// The id for the connection.
    connection_id: ConnectionId,

    /// The driver for the connection.
    driver: Arc<dyn Driver>,

    /// The entity list for the connection.
    entities: Vec<Entity>,

    /// Tracks the locations of entities within the full list by id.
    entity_map: HashMap<SharedString, usize>,
}

impl OpenConnection {
    /// Creates a new [`OpenConnection`].
    pub fn new(connection_id: ConnectionId, driver: Arc<dyn Driver>, entities: Vec<Entity>) -> Self {
        let mut entity_map = HashMap::new();
        for (idx, entity) in entities.iter().enumerate() {
            entity_map.insert(SharedString::from(entity.id()), idx);
        }

        Self {
            connection_id,
            driver,
            entities,
            entity_map,
        }
    }

    /// Returns the color for the connection.
    pub fn color(&self, cx: &App) -> Option<Hsla> {
        if let Some(color) = &self.connection(cx).color {
            Hsla::parse_hex(color).ok()
        } else {
            None
        }
    }

    /// Returns the underlying [`Connection`].
    pub fn connection<'a>(&self, cx: &'a App) -> &'a Connection {
        ConnectionManager::global(cx).connection(&self.connection_id)
    }

    /// Returns the driver for the connection.
    pub fn driver(&self) -> Arc<dyn Driver> {
        self.driver.clone()
    }

    /// Returns the entity list for the connection.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Returns the entity with the given id.
    pub fn entity(&self, id: &SharedString) -> &Entity {
        let idx = self.entity_map[id];
        &self.entities[idx]
    }

    /// Returns the [`ConnectionId`] for the connection.
    pub fn id(&self) -> ConnectionId {
        self.connection_id
    }

    /// Returns the name of the connection.
    pub fn name<'a>(&self, cx: &'a App) -> &'a str {
        &self.connection(cx).name
    }
}
