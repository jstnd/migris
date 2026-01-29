use futures_util::StreamExt;

use crate::{
    BirbError, BirbResult, Column, Connector, ConnectorData, ReadOptions, Row, WriteOptions,
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
    async fn connect(&mut self) -> BirbResult<()> {
        // Nothing to connect to for files.
        Ok(())
    }

    async fn read<'a>(&mut self, _options: &'a ReadOptions) -> BirbResult<ConnectorData<'a>> {
        let mut reader = csv::Reader::from_path(&self.path)
            .map_err(|err| BirbError::FileOpenFailed(err.to_string()))?;

        let headers = reader
            .headers()
            .map_err(|err| BirbError::FileReadFailed(err.to_string()))?;

        let mut columns = Vec::new();
        for (ordinal, name) in headers.iter().enumerate() {
            columns.push(Column::from_csv(name, ordinal));
        }

        let stream = futures_util::stream::iter(reader.into_records().map(|result| {
            result
                .map_err(|err| BirbError::FileReadFailed(err.to_string()))
                .map(Row::from_csv)
        }));

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a>,
        _options: WriteOptions,
    ) -> BirbResult<()> {
        let mut writer = csv::Writer::from_path(&self.path)
            .map_err(|err| BirbError::FileOpenFailed(err.to_string()))?;

        //
        let headers = data.columns.iter().map(|c| &c.name);
        writer
            .write_record(headers)
            .map_err(|err| BirbError::FileWriteFailed(err.to_string()))?;

        //
        let mut stream = data.stream;
        while let Some(row) = stream.next().await {
            let record = row?.into_csv()?;
            writer
                .write_record(&record)
                .map_err(|err| BirbError::FileWriteFailed(err.to_string()))?;
        }

        writer
            .flush()
            .map_err(|err| BirbError::FileWriteFailed(err.to_string()))?;

        Ok(())
    }
}
