pub mod common;
mod connectors;
mod options;
mod schema;
mod value;

pub use connectors::{Connector, ConnectorData, ConnectorKind};
pub use options::{ReadOptions, WriteOptions};
pub use schema::{Column, ColumnFlag, ColumnType, Row, Table};
pub use value::Value;

pub mod csv {
    pub use crate::connectors::csv::{CsvConnector, CsvDataType};
}

pub mod mysql {
    pub use crate::connectors::mysql::{MySqlConnector, MySqlDataType};
}

type MigrisResult<T> = Result<T, MigrisError>;

#[derive(thiserror::Error, Debug)]
pub enum MigrisError {
    #[error("Error encountered: {0}")]
    GeneralError(String),

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

    #[error("Value error encountered: {0}")]
    ValueError(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum FileType {
    Csv,
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::Csv => "csv",
        };

        write!(f, "{}", display)
    }
}

pub fn connector_from_str(str: &str) -> Option<Box<dyn Connector>> {
    if str.starts_with("mysql://") {
        Some(Box::new(mysql::MySqlConnector::new(str)))
    } else if let Some(file_type) = common::get_file_type(&str) {
        match file_type {
            FileType::Csv => Some(Box::new(csv::CsvConnector::new(str))),
        }
    } else {
        None
    }
}
