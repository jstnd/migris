use std::{collections::HashMap, pin::Pin, sync::Arc};

use futures_util::Stream;

use crate::{Column, MigrisResult, Row};

pub struct QueryData {
    columns: Vec<Column>,

    /// Tracks the indexes of columns within the full column list.
    column_map: HashMap<String, usize>,

    rows: Vec<Row>,
}

impl QueryData {
    pub fn new(columns: Vec<Column>, rows: Vec<Row>) -> Self {
        let column_map = columns
            .iter()
            .enumerate()
            .map(|(idx, column)| (column.name.clone(), idx))
            .collect();

        Self {
            columns,
            column_map,
            rows,
        }
    }

    pub fn columns(&self) -> &Vec<Column> {
        &self.columns
    }

    /// Returns the index of the column with the given name.
    pub fn column_index(&self, name: &str) -> usize {
        self.column_map[name]
    }

    pub fn rows(&self) -> &Vec<Row> {
        &self.rows
    }
}

pub struct QueryResult {
    /// The data returned from the query.
    pub data: Arc<QueryData>,

    /// The execution time of the query in milliseconds.
    pub execute_time: u128,

    /// The optional stream where the data will be sourced from.
    pub stream: Option<Pin<Box<dyn Stream<Item = MigrisResult<Row>> + Send>>>,
}

impl QueryResult {
    /// Extends the data stored in the result with the given rows.
    pub fn extend(&mut self, rows: Vec<Row>) {
        if let Some(data) = Arc::get_mut(&mut self.data) {
            data.rows.extend(rows);
        }
    }
}
