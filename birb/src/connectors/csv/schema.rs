use csv::StringRecord;

use crate::{BirbError, Column, ColumnFlag, Row, Value};

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

    pub fn into_csv(self) -> Result<StringRecord, BirbError> {
        let mut record = StringRecord::new();

        for value in self.values {
            record.push_field(&value.to_string()?);
        }

        Ok(record)
    }
}
