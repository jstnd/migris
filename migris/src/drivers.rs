use crate::{Entity, MigrisResult, data::QueryResult};

pub(crate) mod mysql;

#[async_trait::async_trait]
pub trait Driver: Send + Sync {
    async fn entities(&self) -> MigrisResult<Vec<Entity>>;
    async fn query(&self, query: &str) -> MigrisResult<QueryResult>;
    async fn query_stream(&self, query: String) -> MigrisResult<QueryResult>;
}
