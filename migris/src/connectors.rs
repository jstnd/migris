use std::pin::Pin;

use futures_util::Stream;

use crate::{Column, MigrisResult, ReadOptions, Row, Table, WriteOptions};

pub(crate) mod csv;
pub(crate) mod mysql;

type RowStream<'a> = Pin<Box<dyn Stream<Item = MigrisResult<Row>> + Send + 'a>>;

pub struct ConnectorData<'a> {
    pub columns: Vec<Column>,
    pub stream: RowStream<'a>,
}

impl<'a> ConnectorData<'a> {
    pub fn new(columns: Vec<Column>, stream: RowStream<'a>) -> Self {
        Self { columns, stream }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectorKind {
    Database,
    File,
}

#[async_trait::async_trait]
pub trait Connector: Send {
    fn kind(&self) -> ConnectorKind;

    async fn read<'a>(&mut self, options: &'a ReadOptions) -> MigrisResult<ConnectorData<'a>>;

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a>,
        options: &WriteOptions,
    ) -> MigrisResult<()>;

    async fn tables(&mut self) -> MigrisResult<Vec<Table>> {
        // Default implementation (connector kinds such as files will not use this function)
        Ok(vec![])
    }
}
