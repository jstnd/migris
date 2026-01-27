use std::pin::Pin;

use futures_util::Stream;

use crate::{BirbResult, Column, Row, WriteOptions};

pub(crate) mod csv {
    pub(crate) mod connector;
    pub(crate) mod schema;
}

pub(crate) mod mysql {
    pub(crate) mod connector;
    pub(crate) mod schema;
    pub(crate) mod value;
}

type RowStream<'a> = Pin<Box<dyn Stream<Item = BirbResult<Row>> + Send + 'a>>;

pub struct ConnectorData<'a, T>
where
    T: Column,
{
    pub columns: Vec<T>,
    pub stream: RowStream<'a>,
}

impl<'a, T> ConnectorData<'a, T>
where
    T: Column,
{
    pub fn new(columns: Vec<T>, stream: RowStream<'a>) -> Self {
        Self { columns, stream }
    }
}

#[async_trait::async_trait]
pub trait Connector {
    type Column: Column;

    async fn connect(&mut self) -> BirbResult<()>;

    async fn read<'a>(&mut self, query: &'a str) -> BirbResult<ConnectorData<'a, Self::Column>>;

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a, Self::Column>,
        options: WriteOptions,
    ) -> BirbResult<()>;
}
