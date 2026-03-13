use crate::{MigrisError, MigrisResult, mysql::MySqlConnector};

#[async_trait::async_trait]
pub trait Driver {
    async fn entities(&mut self) -> MigrisResult<Vec<Entity>>;
}

#[derive(Debug, sqlx::FromRow)]
pub struct Entity {
    pub name: String,
    pub kind: EntityKind,
}

#[derive(Debug, sqlx::Decode, sqlx::Encode)]
#[sqlx(rename_all = "lowercase")]
pub enum EntityKind {
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
    async fn entities(&mut self) -> MigrisResult<Vec<Entity>> {
        let pool = self.connect().await?;
        let query = r#"
            SELECT
                TABLE_NAME AS `name`,
                IF(TABLE_TYPE = 'BASE TABLE', 'table', 'view') AS `kind`
            FROM information_schema.TABLES
        "#;

        let entities = sqlx::query_as::<sqlx::MySql, Entity>(query)
            .fetch_all(pool)
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

        Ok(entities)
    }
}
