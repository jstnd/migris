use std::pin::Pin;

use futures_lite::Stream;

use crate::{Column, MigrisResult, Row};

pub struct QueryData {
    columns: Vec<Column>,
    rows: Vec<Row>,
}

impl QueryData {
    pub fn new(columns: Vec<Column>, rows: Vec<Row>) -> Self {
        Self { columns, rows }
    }

    pub fn columns(&self) -> &Vec<Column> {
        &self.columns
    }

    pub fn rows(&self) -> &Vec<Row> {
        &self.rows
    }

    pub fn push_row(&mut self, row: Row) {
        self.rows.push(row);
    }
}

pub struct QueryResult {
    /// The data returned from the query.
    pub data: QueryData,

    /// The execution time of the query in milliseconds.
    pub execute_time: u128,

    /// The optional stream where the data will be sourced from.
    pub stream: Option<Pin<Box<dyn Stream<Item = MigrisResult<Row>> + Send>>>,
}
