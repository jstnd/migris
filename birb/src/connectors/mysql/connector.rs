use std::pin::Pin;

use futures_util::StreamExt;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row as SqlxRow};

use crate::{
    BirbError, BirbResult, Column, Connector, ConnectorData, ReadOptions, Row, WriteOptions,
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
        // Validate the given write options for fields that are required.
        validate_write_options(&options)?;

        let pool = self.connect().await?;
        let mut txn = pool
            .begin()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

        let mut stream = data.stream.enumerate();
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
            "INSERT INTO {}.{} VALUES ",
            options.table_schema.unwrap(),
            options.table_name.unwrap()
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
                let query = builder.build();
                query
                    .execute(&mut *txn)
                    .await
                    .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;

                builder.reset();
                current_rows_in_txn = 0;
            }
        }

        if current_rows_in_txn > 0 {
            let query = builder.build();
            query
                .execute(&mut *txn)
                .await
                .map_err(|err| BirbError::DatabaseWriteFailed(err.to_string()))?;
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

fn validate_write_options(options: &WriteOptions) -> BirbResult<()> {
    if options.table_name.is_none() || options.table_schema.is_none() {
        return Err(BirbError::InvalidOption(
            "table schema and name are required when writing to database".into(),
        ));
    }

    Ok(())
}
