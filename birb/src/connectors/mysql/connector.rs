use futures_util::{Stream, StreamExt};

use crate::{BirbError, Connector, Row, mysql::MySqlColumn};

pub struct MySqlConnector {
    identifier: String,
    pub pool: Option<sqlx::MySqlPool>,
}

impl MySqlConnector {
    pub fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.into(),
            pool: None,
        }
    }
}

impl Connector for MySqlConnector {
    type Column = MySqlColumn;

    async fn connect(&mut self) -> Result<(), BirbError> {
        self.pool = Some(
            sqlx::MySqlPool::connect(&self.identifier)
                .await
                .map_err(|err| BirbError::DatabaseConnectFailed {
                    identifier: self.identifier.clone(),
                    message: err.to_string(),
                })?,
        );

        Ok(())
    }

    fn read_data(
        &self,
        query: &str,
    ) -> Result<impl Stream<Item = Result<Row<Self::Column>, BirbError>>, BirbError> {
        let Some(pool) = self.pool.as_ref() else {
            return Err(BirbError::DatabaseReadFailed {
                identifier: self.identifier.clone(),
                message: "no connection was made to the database before attempting to read data"
                    .into(),
            });
        };

        Ok(sqlx::query(query).fetch(pool).map(|row| {
            row.map_err(|err| BirbError::DatabaseReadFailed {
                identifier: self.identifier.clone(),
                message: err.to_string(),
            })
            .and_then(Row::<Self::Column>::from)
        }))
    }
}
