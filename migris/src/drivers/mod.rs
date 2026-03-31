use crate::{Entity, MigrisResult, QueryResult};

pub(crate) mod mysql;

#[async_trait::async_trait]
pub trait Driver: Send + Sync {
    async fn entities(&self) -> MigrisResult<Vec<Entity>>;
    async fn query(&self, query: &str) -> MigrisResult<QueryResult>;
}
