mod connectors;
mod schema;
mod value;

pub use connectors::Connector;
pub use schema::Column;
pub use schema::ColumnFlag;
pub use schema::Row;
pub use value::Value;

pub mod mysql {
    pub use crate::connectors::mysql::connector::MySqlConnector;
    pub use crate::connectors::mysql::schema::MySqlColumn;
    pub use crate::connectors::mysql::schema::MySqlColumnType;
}

#[derive(thiserror::Error, Debug)]
pub enum BirbError {
    #[error("failed to connect to database at {identifier}: {message}")]
    DatabaseConnectFailed { identifier: String, message: String },

    #[error("failed to read from database at {identifier}: {message}")]
    DatabaseReadFailed { identifier: String, message: String },

    #[error("failed to read value: {message}")]
    ValueReadFailed { message: String },
}
