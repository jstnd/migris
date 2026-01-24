mod connectors;
mod options;
mod schema;
mod util;
mod value;

pub use connectors::{Connector, ConnectorData};
pub use options::WriteOptions;
pub use schema::{Column, ColumnFlag, Row};
pub use value::Value;

pub mod csv {
    pub use crate::connectors::csv::connector::CsvConnector;
    pub use crate::connectors::csv::schema::{CsvColumn, CsvColumnType};
}

pub mod mysql {
    pub use crate::connectors::mysql::connector::MySqlConnector;
    pub use crate::connectors::mysql::schema::{MySqlColumn, MySqlColumnType};
}

type BirbResult<T> = Result<T, BirbError>;

#[derive(thiserror::Error, Debug)]
pub enum BirbError {
    #[error("failed to connect to database: {message}")]
    DatabaseConnectFailed { message: String },

    #[error("failed to read from database: {message}")]
    DatabaseReadFailed { message: String },

    #[error("failed to write to database: {message}")]
    DatabaseWriteFailed { message: String },

    #[error("failed to open file: {message}")]
    FileOpenFailed { message: String },

    #[error("failed to read from file: {message}")]
    FileReadFailed { message: String },

    #[error("failed to write to file: {message}")]
    FileWriteFailed { message: String },

    #[error("value error encountered: {message}")]
    ValueError { message: String },
}
