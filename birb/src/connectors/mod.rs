use std::pin::Pin;

use futures_util::Stream;

use crate::{BirbResult, Column, ReadOptions, Row, Table, WriteOptions};

pub(crate) mod csv {
    pub(crate) mod connector;
    pub(crate) mod schema;
}

pub(crate) mod mysql {
    pub(crate) mod connector;
    pub(crate) mod schema;
    pub(crate) mod value;
}

#[async_trait::async_trait]
pub trait Connector: Send {
    fn kind(&self) -> ConnectorKind;

    async fn tables(&mut self, _schema: &str) -> BirbResult<Vec<Table>> {
        Ok(vec![])
    }

    async fn read<'a>(&mut self, options: &'a ReadOptions) -> BirbResult<ConnectorData<'a>>;

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a>,
        options: &WriteOptions,
    ) -> BirbResult<()>;
}

type RowStream<'a> = Pin<Box<dyn Stream<Item = BirbResult<Row>> + Send + 'a>>;

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
