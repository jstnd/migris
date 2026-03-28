use crate::{Entity, MigrisResult};

pub(crate) mod mysql;

#[async_trait::async_trait]
pub trait Driver: Send + Sync {
    async fn entities(&self) -> MigrisResult<Vec<Entity>>;
}