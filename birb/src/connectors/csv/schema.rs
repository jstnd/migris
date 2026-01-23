use csv::StringRecord;

use crate::{Column, ColumnFlag, Row, Value};

#[derive(Clone, Debug)]
pub struct CsvColumn {
    flags: Vec<ColumnFlag>,
    name: String,
    ordinal: usize,
    r#type: CsvColumnType,
}

impl CsvColumn {
    pub fn new(name: impl Into<String>, ordinal: usize) -> Self {
        Self {
            flags: Vec::new(),
            name: name.into(),
            ordinal,
            r#type: CsvColumnType::String,
        }
    }
}

impl Column for CsvColumn {
    type Type = CsvColumnType;

    fn flags(&self) -> &Vec<ColumnFlag> {
        &self.flags
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn ordinal(&self) -> usize {
        self.ordinal
    }

    fn r#type(&self) -> Self::Type {
        self.r#type
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CsvColumnType {
    String,
}

impl Row {
    pub fn from_csv(record: StringRecord) -> Self {
        let mut row = Self::new();

        for value in record.iter() {
            row.values.push(Value::String(value.to_string()));
        }

        row
    }
}
