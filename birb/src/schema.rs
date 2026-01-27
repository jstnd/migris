use crate::{Value, csv::CsvColumnType, mysql::MySqlColumnType};

pub trait Column {
    fn flags(&self) -> &Vec<ColumnFlag>;

    fn name(&self) -> &str;

    fn ordinal(&self) -> usize;

    fn r#type(&self) -> ColumnType;

    fn is_unsigned(&self) -> bool {
        self.flags().contains(&ColumnFlag::Unsigned)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnFlag {
    Unsigned,
}

#[derive(Clone, Copy, Debug)]
pub enum ColumnType {
    Csv(CsvColumnType),
    MySql(MySqlColumnType),
}

impl ColumnType {
    pub fn as_mysql(&self) -> MySqlColumnType {
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
