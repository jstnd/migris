use std::{str::FromStr, sync::Arc, time::Instant};

use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use futures_util::StreamExt;
use sqlx::{
    AssertSqlSafe, Column as SqlxColumn, Executor, Row as SqlxRow, SqlSafeStr, Sqlite, SqlitePool,
    TypeInfo, ValueRef,
    sqlite::{
        SqliteColumn, SqliteConnectOptions, SqliteJournalMode, SqliteRow, SqliteSynchronous,
        SqliteValueRef,
    },
};

use crate::{
    Column, ColumnType, Driver, Entity, EntityData, Index, MigrisError, MigrisResult, Row, Value,
    common::decode_sqlx,
    data::{QueryData, QueryResult},
};

pub struct SqliteConnection {
    pool: SqlitePool,
}

impl SqliteConnection {
    /// Creates a new [`SqliteConnection`] with the given path.
    pub async fn new(path: &str) -> MigrisResult<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|err| MigrisError::DatabaseConnectFailed(err.to_string()))?;

        Ok(Self { pool })
    }

    /// Returns a reference to the connection's pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn columns_from_query(&self, query: Arc<str>) -> MigrisResult<Vec<Column>> {
        self.pool
            .describe(AssertSqlSafe(query).into_sql_str())
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?
            .columns()
            .iter()
            .map(Column::try_from)
            .collect()
    }
}

#[async_trait::async_trait]
impl Driver for SqliteConnection {
    async fn entities(&self) -> MigrisResult<Vec<Entity>> {
        Ok(vec![])
    }

    async fn entity_data(&self, _: &Entity) -> MigrisResult<EntityData> {
        Err(MigrisError::GeneralError("".to_string()))
    }

    async fn indexes(&self, _: &Entity) -> MigrisResult<Vec<Index>> {
        Ok(vec![])
    }

    async fn query(&self, query: String) -> MigrisResult<QueryResult> {
        let query: Arc<str> = Arc::from(query);
        let columns = self.columns_from_query(query.clone()).await?;
        let instant = Instant::now();
        let rows = sqlx::query(AssertSqlSafe(query))
            .fetch_all(&self.pool)
            .await
            .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

        let elapsed = instant.elapsed();
        let rows: MigrisResult<Vec<Row>> = rows.iter().map(Row::try_from).collect();

        Ok(QueryResult {
            data: Arc::new(QueryData::new(columns, rows?)),
            execute_time: elapsed.as_millis(),
            stream: None,
        })
    }

    async fn query_stream(&self, query: String) -> MigrisResult<QueryResult> {
        let query: Arc<str> = Arc::from(query);
        let pool = self.pool.clone();
        let columns = self.columns_from_query(query.clone()).await?;
        let stream = async_stream::stream! {
            let mut stream = sqlx::query(AssertSqlSafe(query)).fetch(&pool);

            while let Some(row) = stream.next().await {
                let row = row
                    .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))
                    .and_then(|row| Row::try_from(&row));

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

impl TryFrom<&SqliteColumn> for Column {
    type Error = MigrisError;

    fn try_from(value: &SqliteColumn) -> Result<Self, Self::Error> {
        Ok(Column {
            name: value.name().to_owned(),
            ordinal: value.ordinal(),
            column_type: ColumnType::Sqlite(SqliteDataType::from_str(value.type_info().name())?),
            flags: Vec::new(),
        })
    }
}

impl TryFrom<&SqliteRow> for Row {
    type Error = MigrisError;

    fn try_from(value: &SqliteRow) -> Result<Self, Self::Error> {
        let mut row = Self::new();
        for column in value.columns() {
            let value = value
                .try_get_raw(column.ordinal())
                .map_err(|err| MigrisError::ValueError(err.to_string()))?;

            row.values.push(Value::try_from(value)?);
        }

        Ok(row)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SqliteDataType {
    Blob,
    Boolean,
    Date,
    Datetime,
    Integer,
    Null,
    Numeric,
    Real,
    Text,
    Time,
}

impl FromStr for SqliteDataType {
    type Err = MigrisError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "BLOB" => Self::Blob,
            "BOOLEAN" => Self::Boolean,
            "DATE" => Self::Date,
            "DATETIME" => Self::Datetime,
            "INTEGER" => Self::Integer,
            "NULL" => Self::Null,
            "NUMERIC" => Self::Numeric,
            "REAL" => Self::Real,
            "TEXT" => Self::Text,
            "TIME" => Self::Time,
            _ => {
                return Err(MigrisError::GeneralError(format!(
                    "failed to convert '{}' to sqlite data type",
                    s
                )));
            }
        })
    }
}

impl TryFrom<SqliteValueRef<'_>> for Value {
    type Error = MigrisError;

    fn try_from(value: SqliteValueRef) -> Result<Self, Self::Error> {
        if value.is_null() {
            return Ok(Value::Null);
        }

        Ok(match SqliteDataType::from_str(value.type_info().name())? {
            SqliteDataType::Blob => Value::Bytes(decode_sqlx::<_, Sqlite, _>(value)?),
            SqliteDataType::Boolean => Value::U8(decode_sqlx::<_, Sqlite, _>(value)?),
            SqliteDataType::Date => {
                let date: NaiveDate = decode_sqlx::<_, Sqlite, _>(value)?;
                let date: NaiveDateTime = date.and_hms_opt(0, 0, 0).ok_or(
                    MigrisError::ValueError("failed to convert date to datetime".into()),
                )?;

                Value::Date(Utc.from_utc_datetime(&date))
            }
            SqliteDataType::Datetime => {
                let date: NaiveDateTime = decode_sqlx::<_, Sqlite, _>(value)?;
                Value::Date(Utc.from_utc_datetime(&date))
            }
            SqliteDataType::Integer => Value::I64(decode_sqlx::<_, Sqlite, _>(value)?),
            SqliteDataType::Null => Value::Null,
            SqliteDataType::Numeric => Value::F64(decode_sqlx::<_, Sqlite, _>(value)?),
            SqliteDataType::Real => Value::F64(decode_sqlx::<_, Sqlite, _>(value)?),
            SqliteDataType::Text => Value::String(decode_sqlx::<_, Sqlite, _>(value)?),
            SqliteDataType::Time => Value::Time(decode_sqlx::<_, Sqlite, _>(value)?),
        })
    }
}
