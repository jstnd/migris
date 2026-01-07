use futures_util::Stream;

use crate::{BirbError, Column, Row};

pub(crate) mod mysql {
    pub(crate) mod connector;
    pub(crate) mod schema;
}

pub trait Connector {
    type Column: Column;

    fn connect(&mut self) -> impl std::future::Future<Output = Result<(), BirbError>> + Send;

    fn read_data(
        &self,
        query: &str,
    ) -> Result<impl Stream<Item = Result<Row<Self::Column>, BirbError>>, BirbError>;
}
