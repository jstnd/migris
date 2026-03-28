use sqlx::MySqlPool;

use crate::{Driver, Entity, MigrisError, MigrisResult};

pub struct MySqlConnection {
    url: String,
    pool: MySqlPool,
}

impl MySqlConnection {
    pub async fn new(url: impl Into<String>) -> MigrisResult<Self> {
        let url = url.into();
        let pool = sqlx::MySqlPool::connect(&url)
            .await
            .map_err(|err| MigrisError::DatabaseConnectFailed(err.to_string()))?;

        Ok(Self { url, pool })
    }
}

#[async_trait::async_trait]
impl Driver for MySqlConnection {
    async fn entities(&self) -> MigrisResult<Vec<Entity>> {
        let query = r#"
            SELECT
                SCHEMA_NAME AS `schema`,
                '' AS `name`,
                'schema' AS `kind`
            FROM information_schema.SCHEMATA
            UNION
            SELECT
                TABLE_SCHEMA AS `schema`,
                TABLE_NAME AS `name`,
                IF(TABLE_TYPE = 'BASE TABLE', 'table', 'view') AS `kind`
            FROM information_schema.TABLES
        "#;

        let entities = sqlx::query_as::<sqlx::MySql, Entity>(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

        Ok(entities)
    }
}
