use std::{fs::OpenOptions, path::Path};

use csv::{Reader, StringRecord, Writer};
use futures_util::StreamExt;

use crate::{
    Column, ColumnType, Connector, ConnectorData, ConnectorKind, MigrisError, MigrisResult,
    ReadOptions, Row, Schema, Value, WriteOptions, common,
};

#[derive(Debug)]
pub struct CsvConnector {
    path: String,
}

impl CsvConnector {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait::async_trait]
impl Connector for CsvConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::File
    }

    async fn read<'a>(&mut self, options: &'a ReadOptions) -> MigrisResult<ConnectorData<'a>> {
        let reader = Reader::from_path(&self.path)
            .map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;

        let columns = Schema::from_csv(&self.path, options.infer_schema)?.columns;
        let stream = futures_util::stream::iter(reader.into_records().map(|result| {
            result
                .map_err(|err| MigrisError::FileReadFailed(err.to_string()))
                .map(Row::from_csv)
        }));

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a>,
        options: &WriteOptions,
    ) -> MigrisResult<()> {
        let path = Path::new(&self.path);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(!options.overwrite)
            .truncate(options.overwrite)
            .open(path)
            .map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;

        // Create any missing parent directories in the given path.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;
        }

        let mut writer = Writer::from_writer(file);

        // Only write headers if we're overwriting or the file is empty.
        if options.overwrite || common::is_file_empty(&path) {
            let headers = data.columns.iter().map(|c| &c.name);
            writer
                .write_record(headers)
                .map_err(|err| MigrisError::FileWriteFailed(err.to_string()))?;
        }

        let mut stream = data.stream.enumerate();
        while let Some((idx, row)) = stream.next().await {
            let record = row?.into_csv()?;
            writer
                .write_record(&record)
                .map_err(|err| MigrisError::FileWriteFailed(err.to_string()))?;

            if options.limit != 0 && idx + 1 == options.limit {
                break;
            }
        }

        writer
            .flush()
            .map_err(|err| MigrisError::FileWriteFailed(err.to_string()))?;

        Ok(())
    }

    async fn exists(&mut self, _options: &WriteOptions) -> bool {
        Path::new(&self.path).exists()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CsvDataType {
    Integer { min: i64, max: i64 },
    String(usize),
}

impl Column {
    fn from_csv(name: impl Into<String>, ordinal: usize) -> Self {
        Self {
            name: name.into(),
            ordinal,
            column_type: ColumnType::Csv(CsvDataType::String(0)),
            flags: Vec::new(),
        }
    }
}

impl Row {
    fn from_csv(record: StringRecord) -> Self {
        let mut row = Self::new();

        for value in record.iter() {
            row.values.push(Value::String(value.to_string()));
        }

        row
    }

    fn into_csv(self) -> MigrisResult<StringRecord> {
        let mut record = StringRecord::new();

        for value in self.values {
            record.push_field(&value.to_string());
        }

        Ok(record)
    }
}

impl Schema {
    fn from_csv(path: &str, infer: bool) -> MigrisResult<Self> {
        let mut reader =
            Reader::from_path(path).map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;

        let mut columns: Vec<Column> = reader
            .headers()
            .map_err(|err| MigrisError::FileReadFailed(err.to_string()))?
            .iter()
            .enumerate()
            .map(|(ordinal, name)| Column::from_csv(name, ordinal))
            .collect();

        if infer {
            let mut column_types = vec![CsvDataType::Integer { min: 0, max: 0 }; columns.len()];

            for result in reader.records() {
                let record = result.map_err(|err| MigrisError::FileReadFailed(err.to_string()))?;

                for (idx, value) in record.iter().enumerate() {
                    if let Some(column_type) = column_types.get_mut(idx) {
                        match column_type {
                            CsvDataType::Integer { min, max } => {
                                // Check if the next value in the column is still an integer.
                                if let Ok(int) = value.parse::<i64>() {
                                    if int < *min {
                                        *min = int;
                                    }

                                    if int > *max {
                                        *max = int;
                                    }
                                } else {
                                    // Otherwise, set the column type to a string instead.
                                    let lengths =
                                        [min.to_string().len(), max.to_string().len(), value.len()];

                                    let len = lengths.iter().max().unwrap_or(&0);
                                    *column_type = CsvDataType::String(*len);
                                }
                            }
                            CsvDataType::String(len) => {
                                let length = value.len();

                                if length > *len {
                                    *len = length;
                                }
                            }
                        }
                    }
                }
            }

            // Set the inferred column types back on the original columns.
            for column in columns.iter_mut() {
                column.column_type = ColumnType::Csv(column_types[column.ordinal]);
            }
        }

        Ok(Self { columns })
    }
}
