use crate::BirbError;
use crate::connectors::Connector;

pub struct MySqlConnector {
    identifier: String,
    pool: Option<sqlx::MySqlPool>,
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
}
