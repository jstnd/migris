use std::pin::Pin;

use futures_util::StreamExt;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row as SqlxRow};

use crate::{
    BirbError, BirbResult, Column, Connector, ConnectorData, Row, WriteOptions, mysql::MySqlColumn,
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

    fn pool(&self) -> &MySqlPool {
        // This function should only be called after a connection
        // is already established, so we use unwrap here.
        self.pool.as_ref().unwrap()
    }
}

impl Connector for MySqlConnector {
    type Column = MySqlColumn;

    async fn connect(&mut self) -> BirbResult<()> {
        if self.pool.is_none() {
            self.pool = Some(
                sqlx::MySqlPool::connect(&self.identifier)
                    .await
                    .map_err(|err| BirbError::DatabaseConnectFailed(err.to_string()))?,
            );
        }

        Ok(())
    }

    async fn read<'a>(&mut self, query: &'a str) -> BirbResult<ConnectorData<'a, Self::Column>> {
        // Ensure a connection exists before performing operations.
        self.connect().await?;

        let mut stream = sqlx::query(query).fetch(self.pool()).peekable();
        let mut columns = Vec::new();

        let peekable = Pin::new(&mut stream);
        if let Some(row) = peekable.peek().await {
            let row = row
                .as_ref()
                .map_err(|err| BirbError::DatabaseReadFailed(err.to_string()))?;

            for column in row.columns() {
                columns.push(MySqlColumn::from(column));
            }
        }

        let stream_columns = columns.clone();
        let stream = stream.map(move |row| {
            row.map_err(|err| BirbError::DatabaseReadFailed(err.to_string()))
                .and_then(|row| Row::from_mysql(row, &stream_columns))
        });

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a, T: Column + Send>(
        &mut self,
        data: ConnectorData<'a, T>,
        options: WriteOptions,
    ) -> BirbResult<()> {
        // Validate the given write options for fields that are required.
        validate_write_options(&options)?;

        // Ensure a connection exists before performing operations.
        self.connect().await?;

        let mut txn = self
            .pool()
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

fn validate_write_options(options: &WriteOptions) -> BirbResult<()> {
    Ok(())
}
