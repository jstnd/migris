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
                CsvDataType::Integer { min, max } => match (*min, *max) {
                    (-128.., ..=127) => MySqlDataType::TINYINT,
                    (-32_768.., ..=32_767) => MySqlDataType::SMALLINT,
                    (-8_388_608.., ..=8_388_607) => MySqlDataType::MEDIUMINT,
                    (-2_147_483_648.., ..=2_147_483_647) => MySqlDataType::INT,
                    _ => MySqlDataType::BIGINT,
                },
                CsvDataType::String(len) => {
                    // Round the length up to the next 10.
                    let len = ((len + 9) / 10) * 10;

                    match len {
                        0..=255 => MySqlDataType::VARCHAR(len as u16),
                        256..=65_535 => MySqlDataType::TEXT,
                        65_536..=16_777_215 => MySqlDataType::MEDIUMTEXT,
                        _ => MySqlDataType::LONGTEXT,
                    }
                }
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

#[derive(Debug)]
pub struct Schema {
    pub(crate) columns: Vec<Column>,
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
