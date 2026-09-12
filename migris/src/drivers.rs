use crate::{Entity, MigrisResult, data::QueryResult, entity::EntityData, schema::Index, shared};

pub(crate) mod mysql;
pub(crate) mod sqlite;

#[async_trait::async_trait]
pub trait Driver: Send + Sync {
    async fn entities(&self) -> MigrisResult<Vec<Entity>>;

    /// Returns the loaded data for the given entity.
    async fn entity_data(&self, entity: &Entity) -> MigrisResult<EntityData>;

    /// Returns the indexes associated with the given entity.
    async fn indexes(&self, entity: &Entity) -> MigrisResult<Vec<Index>>;

    async fn query(&self, query: String) -> MigrisResult<QueryResult>;
    async fn query_stream(&self, query: String) -> MigrisResult<QueryResult>;
}

#[derive(Debug, Clone, Copy, Default, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ConnectionKind {
    #[default]
    MySql,
    Sqlite,
}

impl ConnectionKind {
    pub fn default_port(&self) -> u16 {
        match self {
            ConnectionKind::MySql => shared::DEFAULT_MYSQL_PORT,
            ConnectionKind::Sqlite => 0,
        }
    }
}
