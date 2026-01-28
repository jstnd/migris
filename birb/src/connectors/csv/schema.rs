use csv::StringRecord;

use crate::{BirbResult, Column, ColumnType, Row, Value};

impl Column {
    pub fn from_csv(name: impl Into<String>, ordinal: usize) -> Self {
        Self {
            column_type: ColumnType::Csv(CsvColumnType::String),
            flags: Vec::new(),
            name: name.into(),
            ordinal,
        }
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

    pub fn into_csv(self) -> BirbResult<StringRecord> {
        let mut record = StringRecord::new();

        for value in self.values {
            record.push_field(&value.to_string()?);
        }

        Ok(record)
    }
}
