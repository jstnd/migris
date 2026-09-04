use crate::{Entity, MigrisResult, data::QueryResult, entity::EntityData, schema::Index};

pub(crate) mod mysql;

#[async_trait::async_trait]
pub trait Driver: Send + Sync {
    async fn entities(&self) -> MigrisResult<Vec<Entity>>;

    /// Returns the loaded data for the given entity.
    async fn entity_data(&self, entity: &Entity) -> MigrisResult<EntityData>;

    /// Returns the indexes associated with the given entity.
    async fn indexes(&self, entity: &Entity) -> MigrisResult<Vec<Index>>;

    async fn query(&self, query: &str) -> MigrisResult<QueryResult>;
    async fn query_stream(&self, query: String) -> MigrisResult<QueryResult>;
}
