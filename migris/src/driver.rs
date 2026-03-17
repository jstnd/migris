use crate::{MigrisError, MigrisResult, mysql::MySqlConnector};

#[async_trait::async_trait]
pub trait Driver: std::fmt::Debug + Send + Sync {
    async fn entities(&self) -> MigrisResult<Vec<Entity>>;
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Entity {
    pub schema: String,
    pub name: String,
    pub kind: EntityKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Decode, sqlx::Encode)]
#[sqlx(rename_all = "lowercase")]
pub enum EntityKind {
    Schema,
    Table,
    View,
}

impl<DB> sqlx::Type<DB> for EntityKind
where
    DB: sqlx::Database,
    String: sqlx::Type<DB>,
{
    fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
        <String as sqlx::Type<DB>>::compatible(ty)
    }

    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<DB>>::type_info()
    }
}

#[async_trait::async_trait]
impl Driver for MySqlConnector {
    async fn entities(&self) -> MigrisResult<Vec<Entity>> {
        let query = r#"
            SELECT
                TABLE_SCHEMA AS `schema`,
                TABLE_NAME AS `name`,
                IF(TABLE_TYPE = 'BASE TABLE', 'table', 'view') AS `kind`
            FROM information_schema.TABLES
        "#;

        let entities = sqlx::query_as::<sqlx::MySql, Entity>(query)
            .fetch_all(self.pool.as_ref().unwrap())
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

        Ok(entities)
    }
}
