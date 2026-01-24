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

pub trait Connector {
    type Column: Column;

    fn connect(&mut self) -> impl Future<Output = BirbResult<()>> + Send;

    fn read<'a>(
        &mut self,
        query: &'a str,
    ) -> impl Future<Output = BirbResult<ConnectorData<'a, Self::Column>>> + Send;

    fn write<'a, T: Column + Send>(
        &mut self,
        data: ConnectorData<'a, T>,
        options: WriteOptions,
    ) -> impl Future<Output = BirbResult<()>> + Send;
}
