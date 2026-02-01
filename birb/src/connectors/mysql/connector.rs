use std::pin::Pin;

use futures_util::StreamExt;
use sqlx::{
    MySql, MySqlPool, QueryBuilder, Row as SqlxRow, Transaction, mysql::MySqlArguments,
    query::Query,
};

use crate::{
    BirbError, BirbResult, Column, Connector, ConnectorData, ReadOptions, Row, WriteOptions,
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

    async fn execute_query(
        &self,
        query: Query<'_, MySql, MySqlArguments>,
        txn: &mut Transaction<'_, MySql>,
    ) -> BirbResult<()> {
        query
            .execute(txn.as_mut())
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

        Ok(())
    }

    async fn create_table(
        &self,
        table_schema: &str,
        table_name: &str,
        columns: &[Column],
        txn: &mut Transaction<'_, MySql>,
    ) -> BirbResult<()> {
        let query = format!("CREATE SCHEMA IF NOT EXISTS {}", table_schema);
        self.execute_query(sqlx::query(&query), txn).await?;

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
        self.execute_query(builder.build(), txn).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Connector for MySqlConnector {
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
                columns.push(Column::from_mysql(column));
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
        options: WriteOptions,
    ) -> BirbResult<()> {
        let pool = self.connect().await?;
        let mut txn = pool
            .begin()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

        // Determine table schema and name, using defaults if needed.
        let table_schema = options.table_schema.as_deref().unwrap_or(DEFAULT_SCHEMA);
        let table_name = options.table_name.unwrap_or_else(util::generate_table_name);

        // Create the table if it doesn't already exist.
        self.create_table(table_schema, &table_name, &data.columns, &mut txn)
            .await?;

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
                self.execute_query(builder.build(), &mut txn).await?;
                builder.reset();
                current_rows_in_txn = 0;
            }
        }

        if current_rows_in_txn > 0 {
            self.execute_query(builder.build(), &mut txn).await?;
        }

        txn.commit()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

        Ok(())
    }
}

fn validate_read_options(options: &ReadOptions) -> BirbResult<()> {
    if options.query.is_none() {
        return Err(BirbError::InvalidOption(
            "query is required when reading from database".into(),
        ));
    }

    Ok(())
}
