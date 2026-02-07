use std::{pin::Pin, str::FromStr};

use futures_util::StreamExt;
use sqlx::{
    Executor, MySql, MySqlPool, QueryBuilder, Row as SqlxRow,
    mysql::{MySqlArguments, MySqlConnectOptions},
    query::Query,
};

use crate::{
    BirbError, BirbResult, Column, Connector, ConnectorData, ConnectorKind, ReadOptions, Row,
    Table, WriteOptions,
    util::{self, DEFAULT_SCHEMA},
};

const MYSQL_MAX_PARAMETERS: usize = 65535;

pub struct MySqlConnector {
    identifier: String,
    pool: Option<MySqlPool>,
}

impl MySqlConnector {
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            pool: None,
        }
    }

    async fn connect(&mut self) -> BirbResult<&MySqlPool> {
        if self.pool.is_none() {
            self.pool = Some(
                sqlx::MySqlPool::connect(&self.identifier)
                    .await
                    .map_err(|err| BirbError::DatabaseConnectFailed(err.to_string()))?,
            );
        }

        Ok(self.pool.as_ref().unwrap())
    }
}

#[async_trait::async_trait]
impl Connector for MySqlConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::Database
    }

    async fn tables(&mut self) -> BirbResult<Vec<Table>> {
        if let Some(schema) = schema_from_identifier(&self.identifier) {
            let pool = self.connect().await?;
            let query = r#"
                SELECT
                    TABLE_SCHEMA AS `schema`, TABLE_NAME AS `name`
                FROM information_schema.TABLES
                WHERE TABLE_SCHEMA = ?
            "#;

            let tables = sqlx::query_as::<_, Table>(query)
                .bind(schema)
                .fetch_all(pool)
                .await
                .map_err(|err| BirbError::DatabaseReadFailed(err.to_string()))?;

            Ok(tables)
        } else {
            Ok(vec![])
        }
    }

    async fn read<'a>(&mut self, options: &'a ReadOptions) -> BirbResult<ConnectorData<'a>> {
        // Validate the given read options for fields that are required.
        validate_read_options(options)?;

        let pool = self.connect().await?;
        let mut stream = sqlx::query(options.query.as_ref().unwrap())
            .fetch(pool)
            .peekable();

        let mut columns = Vec::new();
        let peekable = Pin::new(&mut stream);

        if let Some(row) = peekable.peek().await {
            let row = row
                .as_ref()
                .map_err(|err| BirbError::DatabaseReadFailed(err.to_string()))?;

            for column in row.columns() {
                columns.push(Column::from_mysql(column)?);
            }
        }

        let stream_columns = columns.clone();
        let stream = stream.map(move |row| {
            row.map_err(|err| BirbError::DatabaseReadFailed(err.to_string()))
                .and_then(|row| Row::from_mysql(row, &stream_columns))
        });

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a>,
        options: &WriteOptions,
    ) -> BirbResult<()> {
        // Determine table schema and name, using defaults if needed.
        let generated = util::generate_name();
        let table_name = options.table_name.as_deref().unwrap_or(&generated);
        let table_schema = if let Some(schema) = schema_from_identifier(&self.identifier) {
            schema
        } else if let Some(schema) = &options.table_schema {
            schema.clone()
        } else {
            DEFAULT_SCHEMA.to_string()
        };

        let pool = self.connect().await?;
        let mut txn = pool
            .begin()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

        // Create the table if it doesn't already exist.
        create_table(&table_schema, table_name, &data.columns, pool).await?;

        let mut stream = data.stream.enumerate();
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
            "INSERT INTO {}.{} VALUES ",
            table_schema, table_name
        ));

        let mut rows_per_txn = 0;
        let mut current_rows_in_txn = 0;

        while let Some((idx, row)) = stream.next().await {
            let row = row?;

            // Perform data extraction using first row encountered.
            if idx == 0 {
                // Determine the maximum number of rows we can fit into a transaction.
                rows_per_txn = MYSQL_MAX_PARAMETERS / data.columns.len();
            }

            if current_rows_in_txn > 0 {
                builder.push(", ");
            }

            builder.push("(");
            let mut separated = builder.separated(", ");

            for value in row.values {
                separated.push_bind(value);
            }

            separated.push_unseparated(")");
            current_rows_in_txn += 1;

            if current_rows_in_txn == rows_per_txn {
                execute_query(builder.build(), &mut *txn).await?;
                builder.reset();
                current_rows_in_txn = 0;
            }
        }

        if current_rows_in_txn > 0 {
            execute_query(builder.build(), &mut *txn).await?;
        }

        txn.commit()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

        Ok(())
    }
}

async fn create_table(
    table_schema: &str,
    table_name: &str,
    columns: &[Column],
    pool: &MySqlPool,
) -> BirbResult<()> {
    let query = format!("CREATE SCHEMA IF NOT EXISTS {}", table_schema);
    execute_query(sqlx::query(&query), pool).await?;

    let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
        "CREATE TABLE IF NOT EXISTS {0}.{1} (",
        table_schema, table_name
    ));

    let mut separated = builder.separated(", ");
    for column in columns {
        let mut definition = format!("`{}` {}", column.name, column.column_type.as_mysql());

        if column.is_unsigned() {
            definition.push_str(" UNSIGNED");
        }

        separated.push(definition);
    }

    builder.push(")");
    execute_query(builder.build(), pool).await?;

    Ok(())
}

async fn execute_query<'e, E>(
    query: Query<'_, MySql, MySqlArguments>,
    executor: E,
) -> BirbResult<()>
where
    E: Executor<'e, Database = MySql>,
{
    query
        .execute(executor)
        .await
        .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

    Ok(())
}

fn schema_from_identifier(identifier: &str) -> Option<String> {
    if let Ok(options) = MySqlConnectOptions::from_str(identifier)
        && let Some(schema) = options.get_database()
    {
        return Some(schema.to_string());
    }

    None
}

fn validate_read_options(options: &ReadOptions) -> BirbResult<()> {
    if options.query.is_none() {
        return Err(BirbError::InvalidOption(
            "query is required when reading from database".into(),
        ));
    }

    Ok(())
}
