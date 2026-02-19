use std::{fs::OpenOptions, path::Path};

use futures_util::StreamExt;

use crate::{
    Column, Connector, ConnectorData, ConnectorKind, MigrisError, MigrisResult, ReadOptions, Row,
    WriteOptions, common,
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
        _options: &WriteOptions,
    ) -> MigrisResult<()> {
        let path = Path::new(&self.path);
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;

        // Create any missing parent directories in the given path.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| MigrisError::FileOpenFailed(err.to_string()))?;
        }

        let mut writer = csv::Writer::from_writer(file);

        // Only write headers if the file is empty.
        if common::is_file_empty(&path) {
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
}
