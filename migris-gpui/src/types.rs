use std::{collections::HashMap, sync::Arc};

use gpui_kit::SharedString;
use migris::{Entity, drivers::Driver};

use crate::connections::{Connection, ConnectionId};

pub struct OpenConnection {
    /// The information for the connection.
    connection: Connection,

    /// The driver for the connection.
    driver: Arc<dyn Driver>,

    /// The entity list for the connection.
    entities: Vec<Entity>,

    /// Tracks the locations of entities within the full list by id.
    entity_map: HashMap<SharedString, usize>,
}

impl OpenConnection {
    /// Creates a new [`OpenConnection`].
    pub fn new(connection: Connection, driver: Arc<dyn Driver>, entities: Vec<Entity>) -> Self {
        let mut entity_map = HashMap::new();
        for (idx, entity) in entities.iter().enumerate() {
            entity_map.insert(SharedString::from(entity.id()), idx);
        }

        Self {
            connection,
            driver,
            entities,
            entity_map,
        }
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
        self.connection.id
    }

    /// Returns the name of the connection.
    pub fn name(&self) -> &str {
        &self.connection.name
    }
}
