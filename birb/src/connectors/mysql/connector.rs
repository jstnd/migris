use futures_util::StreamExt;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use crate::{BirbError, Connector, Row, WriteOptions, connectors::RowStream, mysql::MySqlColumn};

const MYSQL_MAX_PARAMETERS: usize = 65535;

pub struct MySqlConnector {
    identifier: String,
    pub pool: Option<MySqlPool>,
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
            .ok_or_else(|| BirbError::DatabaseInteractBeforeConnect {
                identifier: self.identifier.to_string(),
            })
    }
}

impl Connector for MySqlConnector {
    type Column = MySqlColumn;

    async fn connect(&mut self) -> Result<(), BirbError> {
        self.pool = Some(
            sqlx::MySqlPool::connect(&self.identifier)
                .await
                .map_err(|err| BirbError::DatabaseConnectFailed {
                    identifier: self.identifier.to_string(),
                    message: err.to_string(),
                })?,
        );

        Ok(())
    }

    fn read<'a>(&'a self, query: &'a str) -> Result<RowStream<'a, Self::Column>, BirbError> {
        let pool = self.pool()?;
        let stream = sqlx::query(query).fetch(pool).map(|row| {
            row.map_err(|err| BirbError::DatabaseReadFailed {
                identifier: self.identifier.to_string(),
                message: err.to_string(),
            })
            .and_then(Row::<Self::Column>::from)
        });

        Ok(Box::pin(stream))
    }

    async fn write<'a>(
        &self,
        stream: RowStream<'a, Self::Column>,
        options: WriteOptions<'a>,
    ) -> Result<(), BirbError> {
        // TODO: validate options

        let pool = self.pool()?;
        let mut txn = pool
            .begin()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed {
                identifier: self.identifier.to_string(),
                message: err.to_string(),
            })?;

        let mut stream = stream.enumerate();
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
                rows_per_txn = MYSQL_MAX_PARAMETERS / row.columns.len();
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
                        identifier: self.identifier.to_string(),
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
                    identifier: self.identifier.to_string(),
                    message: err.to_string(),
                })?;
        }

        txn.commit()
            .await
            .map_err(|err| BirbError::DatabaseWriteFailed {
                identifier: self.identifier.to_string(),
                message: err.to_string(),
            })?;

        Ok(())
    }
}
