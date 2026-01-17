use std::pin::Pin;

use futures_util::Stream;

use crate::{BirbError, Column, Row};

pub(crate) mod mysql {
    pub(crate) mod connector;
    pub(crate) mod schema;
    pub(crate) mod value;
}

type RowStream<'a, T> = Pin<Box<dyn Stream<Item = Result<Row<T>, BirbError>> + Send + 'a>>;

pub trait Connector {
    type Column: Column;

    fn connect(&mut self) -> impl std::future::Future<Output = Result<(), BirbError>> + Send;

    fn read<'a>(&'a self, query: &'a str) -> Result<RowStream<'a, Self::Column>, BirbError>;

    fn write<'a>(
        &self,
        stream: RowStream<'a, Self::Column>,
        options: WriteOptions<'a>,
    ) -> impl std::future::Future<Output = Result<(), BirbError>> + Send;
}

pub struct WriteOptions<'a> {
    pub table_schema: &'a str,
    pub table_name: &'a str,
}
