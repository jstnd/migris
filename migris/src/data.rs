use crate::{Column, Row};

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
}

pub struct QueryResult {
    /// The data returned from the query.
    pub data: QueryData,

    /// The execution time of the query in milliseconds.
    pub execute_time: u128,
}
