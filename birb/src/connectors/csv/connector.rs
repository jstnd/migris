use csv::Reader;

use crate::{BirbError, Connector, ConnectorData, Row, WriteOptions, csv::CsvColumn};

#[derive(Debug)]
pub struct CsvConnector {
    path: String,
}

impl CsvConnector {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Connector for CsvConnector {
    type Column = CsvColumn;

    async fn connect(&mut self) -> Result<(), BirbError> {
        // Nothing to connect to for files.
        Ok(())
    }

    async fn read<'a>(
        &mut self,
        _query: &'a str,
    ) -> Result<ConnectorData<'a, Self::Column>, BirbError> {
        let mut reader =
            Reader::from_path(&self.path).map_err(|err| BirbError::FileOpenFailed {
                message: err.to_string(),
            })?;

        let headers = reader.headers().map_err(|err| BirbError::FileReadFailed {
            message: err.to_string(),
        })?;

        let mut columns = Vec::new();
        for (ordinal, name) in headers.iter().enumerate() {
            columns.push(CsvColumn::new(name, ordinal));
        }

        let stream = futures_util::stream::iter(reader.into_records().map(|result| {
            result
                .map_err(|err| BirbError::FileReadFailed {
                    message: err.to_string(),
                })
                .map(Row::from_csv)
        }));

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a, Self::Column>,
        options: WriteOptions<'a>,
    ) -> Result<(), BirbError> {
        unimplemented!()
    }
}
