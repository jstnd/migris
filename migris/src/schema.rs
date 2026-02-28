use crate::{Value, csv::CsvDataType, mysql::MySqlDataType};

#[derive(Clone, Debug)]
pub struct Column {
    pub(crate) name: String,
    pub(crate) ordinal: usize,
    pub(crate) column_type: ColumnType,
    pub(crate) flags: Vec<ColumnFlag>,
}

impl Column {
    pub fn is_nullable(&self) -> bool {
        self.flags.contains(&ColumnFlag::Nullable)
    }

    pub fn is_unsigned(&self) -> bool {
        self.flags.contains(&ColumnFlag::Unsigned)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnFlag {
    Nullable,
    Unsigned,
}

#[derive(Clone, Debug)]
pub enum ColumnType {
    Csv(CsvDataType),
    MySql(MySqlDataType),
}

impl ColumnType {
    pub fn as_mysql(&self) -> MySqlDataType {
        match self {
            ColumnType::Csv(data_type) => match data_type {
                CsvDataType::String => MySqlDataType::VARCHAR(500),
            },
            ColumnType::MySql(data_type) => data_type.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Row {
    pub values: Vec<Value>,
}

impl Row {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct Table {
    pub schema: String,
    pub name: String,
}

impl Table {
    pub fn new(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
        }
    }
}
