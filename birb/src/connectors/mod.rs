use std::pin::Pin;

use futures_util::Stream;

use crate::{BirbError, Column, Row};

pub(crate) mod csv {
    pub(crate) mod connector;
    pub(crate) mod schema;
}

pub(crate) mod mysql {
    pub(crate) mod connector;
    pub(crate) mod schema;
    pub(crate) mod value;
}

type RowStream<'a> = Pin<Box<dyn Stream<Item = Result<Row, BirbError>> + Send + 'a>>;

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

    fn connect(&mut self) -> impl Future<Output = Result<(), BirbError>> + Send;

    fn read<'a>(
        &mut self,
        query: &'a str,
    ) -> impl Future<Output = Result<ConnectorData<'a, Self::Column>, BirbError>> + Send;

    fn write<'a>(
        &mut self,
        data: ConnectorData<'a, Self::Column>,
        options: WriteOptions<'a>,
    ) -> impl Future<Output = Result<(), BirbError>> + Send;
}

pub struct WriteOptions<'a> {
    pub table_schema: &'a str,
    pub table_name: &'a str,
}
