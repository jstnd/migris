use std::pin::Pin;

use futures_util::Stream;

use crate::{BirbError, Column, Row};

pub(crate) mod mysql {
    pub(crate) mod connector;
    pub(crate) mod schema;
}

type RowStream<'a, T> = Pin<Box<dyn Stream<Item = Result<Row<T>, BirbError>> + 'a>>;

pub trait Connector {
    type Column: Column;

    fn connect(&mut self) -> impl std::future::Future<Output = Result<(), BirbError>> + Send;

    fn read<'a>(&'a self, query: &'a str) -> Result<RowStream<'a, Self::Column>, BirbError>;

    fn write<'a>(&self, stream: RowStream<'a, Self::Column>) -> Result<(), BirbError>;
}
