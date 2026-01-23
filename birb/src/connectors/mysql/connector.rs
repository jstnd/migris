use std::pin::Pin;

use futures_util::StreamExt;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row as SqlxRow};

use crate::{BirbError, Connector, ConnectorData, Row, WriteOptions, mysql::MySqlColumn};

const MYSQL_MAX_PARAMETERS: usize = 65535;

pub struct MySqlConnector {
    identifier: String,
    pool: Option<MySqlPool>,
}

impl MySqlConnector {
    pub fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            pool: None,
        }
    }

    pub(crate) fn pool(&self) -> Result<&MySqlPool, BirbError> {
        self.pool
            .as_ref()
            .ok_or(BirbError::DatabaseInteractBeforeConnect)
    }
}

impl Connector for MySqlConnector {
    type Column = MySqlColumn;

    async fn connect(&mut self) -> Result<(), BirbError> {
        self.pool = Some(
            sqlx::MySqlPool::connect(&self.identifier)
                .await
                .map_err(|err| BirbError::DatabaseConnectFailed {
                    message: err.to_string(),
                })?,
        );

        Ok(())
    }

    async fn read<'a>(&self, query: &'a str) -> Result<ConnectorData<'a, Self::Column>, BirbError> {
        let pool = self.pool()?;
        let mut stream = sqlx::query(query).fetch(pool).peekable();
        let mut columns = Vec::new();

        let peekable = Pin::new(&mut stream);
        if let Some(row) = peekable.peek().await {
            let row = row.as_ref().map_err(|err| BirbError::DatabaseReadFailed {
                message: err.to_string(),
            })?;

            for column in row.columns() {
                columns.push(MySqlColumn::from(column));
            }
        }

        let stream_columns = columns.clone();
        let stream = stream.map(move |row| {
            row.map_err(|err| BirbError::DatabaseReadFailed {
                message: err.to_string(),
            })
            .and_then(|row| Row::from_mysql(row, &stream_columns))
        });

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a>(
        &self,
        data: ConnectorData<'a, Self::Column>,
        options: WriteOptions<'a>,
    ) -> Result<(), BirbError> {
        // TODO: validate options

        let pool = self.pool()?;
        let mut txn = pool
            .begin()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed {
                message: err.to_string(),
            })?;

        let mut stream = data.stream.enumerate();
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
            "INSERT INTO {}.{} VALUES ",
            options.table_schema, options.table_name
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
                    .map_err(|err| BirbError::DatabaseWriteFailed {
                        message: err.to_string(),
                    })?;

                builder.reset();
                current_rows_in_txn = 0;
            }
        }

        if current_rows_in_txn > 0 {
            let query = builder.build();
            query
                .execute(&mut *txn)
                .await
                .map_err(|err| BirbError::DatabaseWriteFailed {
                    message: err.to_string(),
                })?;
        }

        txn.commit()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed {
                message: err.to_string(),
            })?;

        Ok(())
    }
}
