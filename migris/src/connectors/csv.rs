use std::{fs::OpenOptions, path::Path};

use csv::StringRecord;
use futures_util::StreamExt;

use crate::{
    Column, ColumnType, Connector, ConnectorData, ConnectorKind, MigrisError, MigrisResult,
    ReadOptions, Row, Value, WriteOptions, common,
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

    async fn read<'a>(&mut self, _options: &'a ReadOptions) -> MigrisResult<ConnectorData<'a>> {
        let mut reader = csv::Reader::from_path(&self.path)
            .map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;

        let headers = reader
            .headers()
            .map_err(|err| MigrisError::FileReadFailed(err.to_string()))?;

        let mut columns = Vec::new();
        for (ordinal, name) in headers.iter().enumerate() {
            columns.push(Column::from_csv(name, ordinal));
        }

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

        let mut writer = csv::Writer::from_writer(file);

        // Only write headers if we're overwriting or the file is empty.
        if options.overwrite || common::is_file_empty(&path) {
            let headers = data.columns.iter().map(|c| &c.name);
            writer
                .write_record(headers)
                .map_err(|err| MigrisError::FileWriteFailed(err.to_string()))?;
        }

        //
        let mut stream = data.stream;
        while let Some(row) = stream.next().await {
            let record = row?.into_csv()?;
            writer
                .write_record(&record)
                .map_err(|err| MigrisError::FileWriteFailed(err.to_string()))?;
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
    String,
}

impl Column {
    fn from_csv(name: impl Into<String>, ordinal: usize) -> Self {
        Self {
            column_type: ColumnType::Csv(CsvDataType::String),
            flags: Vec::new(),
            name: name.into(),
            ordinal,
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
            record.push_field(&value.to_string()?);
        }

        Ok(record)
    }
}
