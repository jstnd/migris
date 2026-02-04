mod connectors;
mod options;
mod schema;
pub mod util;
mod value;

pub use connectors::{Connector, ConnectorData};
pub use options::{ReadOptions, WriteOptions};
pub use schema::{Column, ColumnFlag, ColumnType, Row};
pub use value::Value;

pub mod csv {
    pub use crate::connectors::csv::connector::CsvConnector;
    pub use crate::connectors::csv::schema::CsvDataType;
}

pub mod mysql {
    pub use crate::connectors::mysql::connector::MySqlConnector;
    pub use crate::connectors::mysql::schema::MySqlDataType;
}

type BirbResult<T> = Result<T, BirbError>;

#[derive(thiserror::Error, Debug)]
pub enum BirbError {
    #[error("Failed to connect to database: {0}")]
    DatabaseConnectFailed(String),

    #[error("Failed to read from database: {0}")]
    DatabaseReadFailed(String),

    #[error("Failed to write to database: {0}")]
    DatabaseWriteFailed(String),

    #[error("Failed to open file: {0}")]
    FileOpenFailed(String),

    #[error("Failed to read from file: {0}")]
    FileReadFailed(String),

    #[error("Failed to write to file: {0}")]
    FileWriteFailed(String),

    #[error("Invalid option: {0}")]
    InvalidOption(String),

    #[error("Unsupported action performed: {0}")]
    UnsupportedAction(String),

    #[error("Value error encountered: {0}")]
    ValueError(String),
}

pub fn connector_from_str(str: &str) -> Option<Box<dyn Connector>> {
    if str.starts_with("mysql://") {
        Some(Box::new(mysql::MySqlConnector::new(str)))
    } else if let Some(extension) = util::get_extension(&str) {
        match extension {
            "csv" => Some(Box::new(csv::CsvConnector::new(str))),
            _ => None,
        }
    } else {
        None
    }
}
