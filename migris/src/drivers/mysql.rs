use std::{collections::HashMap, str::FromStr, sync::Arc, time::Instant};

use futures_util::StreamExt;
use sqlx::{
    Column as SqlxColumn, Executor, MySqlPool, Row as SqlxRow, TypeInfo,
    mysql::{MySqlColumn, MySqlTypeInfo},
};

use crate::{
    Column, ColumnType, Driver, Entity, EntityKind, MigrisError, MigrisResult, Row,
    data::{QueryData, QueryResult},
    entity::{EntityData, TableData},
    mysql::MySqlDataType,
    schema::{Index, IndexKind},
};

pub struct MySqlConnection {
    url: String,
    pool: MySqlPool,
}

impl MySqlConnection {
    /// Creates a new [`MySqlConnection`] with the given connection URL.
    pub async fn new(url: impl Into<String>) -> MigrisResult<Self> {
        let url = url.into();
        let pool = sqlx::MySqlPool::connect(&url)
            .await
            .map_err(|err| MigrisError::DatabaseConnectFailed(err.to_string()))?;

        Ok(Self { url, pool })
    }

    async fn columns_from_query(&self, query: &str) -> MigrisResult<Vec<Column>> {
        self.pool
            .describe(query)
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?
            .columns()
            .iter()
            .map(Column::from_mysql_query)
            .collect()
    }
}

#[async_trait::async_trait]
impl Driver for MySqlConnection {
    async fn entities(&self) -> MigrisResult<Vec<Entity>> {
        let query = r#"
            SELECT
                EVENT_SCHEMA AS `schema`,
                EVENT_NAME AS `name`,
                'event' AS `kind`
            FROM information_schema.EVENTS
            UNION
            SELECT
                ROUTINE_SCHEMA AS `schema`,
                ROUTINE_NAME AS `name`,
                IF(ROUTINE_TYPE = 'FUNCTION', 'function', 'procedure') AS `kind`
            FROM information_schema.ROUTINES
            UNION
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
            UNION
            SELECT
                TRIGGER_SCHEMA AS `schema`,
                TRIGGER_NAME AS `name`,
                'trigger' AS `kind`
            FROM information_schema.TRIGGERS
        "#;

        let entities = sqlx::query_as::<sqlx::MySql, Entity>(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

        Ok(entities)
    }

    async fn entity_data(&self, entity: &Entity) -> MigrisResult<EntityData> {
        let data = match entity.kind {
            EntityKind::Table => EntityData::Table(TableData {
                indexes: self.indexes(entity).await?,
            }),
            _ => {
                return Err(MigrisError::GeneralError(
                    "attempted to retrieve data for unsupported entity kind".to_string(),
                ));
            }
        };

        Ok(data)
    }

    async fn indexes(&self, entity: &Entity) -> MigrisResult<Vec<Index>> {
        let query = r#"
            SELECT
                INDEX_NAME,
                COLUMN_NAME,
                CASE
                    WHEN INDEX_NAME = 'PRIMARY' THEN 'PRIMARY'
                    WHEN NON_UNIQUE = 0 THEN 'UNIQUE'
                    ELSE 'REGULAR'
                END AS INDEX_KIND
            FROM information_schema.STATISTICS
            WHERE
                TABLE_SCHEMA = ? AND
                TABLE_NAME = ?
            ORDER BY
                INDEX_NAME,
                SEQ_IN_INDEX
        "#;

        let mut indexes: HashMap<String, Index> = HashMap::new();
        let mut stream = sqlx::query(query)
            .bind(&entity.schema)
            .bind(&entity.name)
            .fetch(&self.pool);

        while let Some(row) = stream.next().await {
            let row = row.map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;
            let index_name: String = row.get("INDEX_NAME");

            indexes
                .entry(index_name.clone())
                .or_insert(Index::new(
                    IndexKind::from_str(row.get("INDEX_KIND"))?,
                    index_name,
                ))
                .columns
                .push(row.get("COLUMN_NAME"));
        }

        Ok(indexes.into_values().collect())
    }

    async fn query(&self, query: &str) -> MigrisResult<QueryResult> {
        let columns = self.columns_from_query(query).await?;
        let instant = Instant::now();
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

        let elapsed = instant.elapsed();
        let rows: MigrisResult<Vec<Row>> = rows
            .iter()
            .map(|row| Row::from_mysql(row, &columns))
            .collect();

        Ok(QueryResult {
            data: Arc::new(QueryData::new(columns, rows?)),
            execute_time: elapsed.as_millis(),
            stream: None,
        })
    }

    async fn query_stream(&self, query: String) -> MigrisResult<QueryResult> {
        let pool = self.pool.clone();
        let columns = self.columns_from_query(&query).await?;
        let stream_columns = columns.clone();
        let stream = async_stream::stream! {
            let mut stream = sqlx::query(&query).fetch(&pool);

            while let Some(row) = stream.next().await {
                let row = row
                    .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))
                    .and_then(|row| Row::from_mysql(&row, &stream_columns));

                yield row;
            }
        };

        Ok(QueryResult {
            data: Arc::new(QueryData::new(columns, Vec::new())),
            execute_time: 0,
            stream: Some(Box::pin(stream)),
        })
    }
}

impl Column {
    fn from_mysql_query(sqlx_column: &MySqlColumn) -> MigrisResult<Self> {
        Ok(Column {
            name: sqlx_column.name().to_string(),
            ordinal: sqlx_column.ordinal(),
            column_type: ColumnType::MySql(MySqlDataType::from_sqlx(sqlx_column.type_info())?),
            flags: Vec::new(),
        })
    }
}

impl MySqlDataType {
    fn from_sqlx(type_info: &MySqlTypeInfo) -> MigrisResult<Self> {
        let name = type_info.name().trim_end_matches(" UNSIGNED");

        Ok(match name {
            "BIGINT" => Self::BIGINT,
            "BINARY" => Self::BINARY(u8::MAX),
            "BIT" => Self::BIT(u8::MAX),
            "BLOB" => Self::BLOB,
            "BOOLEAN" => Self::TINYINT,
            "CHAR" => Self::CHAR(u8::MAX),
            "DATE" => Self::DATE,
            "DATETIME" => Self::DATETIME,
            "DECIMAL" => Self::DECIMAL(u8::MAX, u8::MAX),
            "DOUBLE" => Self::DOUBLE,
            "ENUM" => Self::ENUM(Vec::new()),
            "FLOAT" => Self::FLOAT,
            "GEOMETRY" => Self::GEOMETRY,
            "INT" => Self::INT,
            "JSON" => Self::JSON,
            "LONGBLOB" => Self::LONGBLOB,
            "LONGTEXT" => Self::LONGTEXT,
            "MEDIUMBLOB" => Self::MEDIUMBLOB,
            "MEDIUMINT" => Self::MEDIUMINT,
            "MEDIUMTEXT" => Self::MEDIUMTEXT,
            "SET" => Self::SET(Vec::new()),
            "SMALLINT" => Self::SMALLINT,
            "TEXT" => Self::TEXT,
            "TIME" => Self::TIME,
            "TIMESTAMP" => Self::TIMESTAMP,
            "TINYBLOB" => Self::TINYBLOB,
            "TINYINT" => Self::TINYINT,
            "TINYTEXT" => Self::TINYTEXT,
            "VARBINARY" => Self::VARBINARY(u16::MAX),
            "VARCHAR" => Self::VARCHAR(u16::MAX),
            "YEAR" => Self::YEAR,
            _ => {
                return Err(MigrisError::GeneralError(format!(
                    "failed to convert '{}' to mysql data type",
                    name
                )));
            }
        })
    }
}
