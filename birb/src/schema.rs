use crate::{Value, csv::CsvDataType, mysql::MySqlDataType};

#[derive(Clone, Debug)]
pub struct Column {
    pub(crate) column_type: ColumnType,
    pub(crate) flags: Vec<ColumnFlag>,
    pub(crate) name: String,
    pub(crate) ordinal: usize,
}

impl Column {
    pub fn is_unsigned(&self) -> bool {
        self.flags.contains(&ColumnFlag::Unsigned)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnFlag {
    Unsigned,
}

#[derive(Clone, Copy, Debug)]
pub enum ColumnType {
    Csv(CsvDataType),
    MySql(MySqlDataType),
}

impl ColumnType {
    pub fn as_mysql(&self) -> MySqlDataType {
        match self {
            ColumnType::Csv(_) => unimplemented!(),
            ColumnType::MySql(column_type) => *column_type,
        }
    }
}

#[derive(Debug)]
pub struct Row {
    pub values: Vec<Value>,
}

impl Row {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}
