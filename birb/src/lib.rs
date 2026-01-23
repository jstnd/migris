mod connectors;
mod schema;
mod util;
mod value;

pub use connectors::{Connector, ConnectorData, WriteOptions};
pub use schema::{Column, ColumnFlag, Row};
pub use value::Value;

pub mod mysql {
    pub use crate::connectors::mysql::connector::MySqlConnector;
    pub use crate::connectors::mysql::schema::{MySqlColumn, MySqlColumnType};
}

#[derive(thiserror::Error, Debug)]
pub enum BirbError {
    #[error("failed to connect to database: {message}")]
    DatabaseConnectFailed { message: String },

    #[error("no connection was made to database before attempting to interact with data")]
    DatabaseInteractBeforeConnect,

    #[error("failed to read from database: {message}")]
    DatabaseReadFailed { message: String },

    #[error("failed to write to database: {message}")]
    DatabaseWriteFailed { message: String },

    #[error("failed to read value: {message}")]
    ValueReadFailed { message: String },
}
