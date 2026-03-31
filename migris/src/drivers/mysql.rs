use std::time::Instant;

use sqlx::{
    Column as SqlxColumn, Executor, MySqlPool, TypeInfo,
    mysql::{MySqlColumn, MySqlTypeInfo},
};

use crate::{
    Column, ColumnType, Driver, Entity, MigrisError, MigrisResult, QueryData, QueryResult, Row,
    mysql::MySqlDataType,
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
            data: QueryData::new(columns, rows?),
            execute_time: elapsed.as_millis(),
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
