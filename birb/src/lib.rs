mod connectors;
mod options;
mod schema;
mod util;
mod value;

pub use connectors::{Connector, ConnectorData};
pub use options::{ReadOptions, WriteOptions};
pub use schema::{Column, ColumnFlag, ColumnType, Row};
pub use value::Value;

pub mod csv {
    pub use crate::connectors::csv::connector::CsvConnector;
    pub use crate::connectors::csv::schema::CsvColumnType;
}

pub mod mysql {
    pub use crate::connectors::mysql::connector::MySqlConnector;
    pub use crate::connectors::mysql::schema::MySqlColumnType;
}

type BirbResult<T> = Result<T, BirbError>;

#[derive(thiserror::Error, Debug)]
pub enum BirbError {
    #[error("failed to connect to database: {0}")]
    DatabaseConnectFailed(String),

    #[error("failed to read from database: {0}")]
    DatabaseReadFailed(String),

    #[error("failed to write to database: {0}")]
    DatabaseWriteFailed(String),

    #[error("failed to open file: {0}")]
    FileOpenFailed(String),

    #[error("failed to read from file: {0}")]
    FileReadFailed(String),

    #[error("failed to write to file: {0}")]
    FileWriteFailed(String),

    #[error("invalid option: {0}")]
    InvalidOption(String),

    #[error("value error encountered: {0}")]
    ValueError(String),
}

pub fn connector_from_str(str: &str) -> Option<Box<dyn Connector>> {
    if str.starts_with("mysql://") {
        Some(Box::new(mysql::MySqlConnector::new(str)))
    } else if let Some(extension) = util::get_file_extension(str) {
        match extension {
            "csv" => Some(Box::new(csv::CsvConnector::new(str))),
            _ => None,
        }
    } else {
        None
    }
}
