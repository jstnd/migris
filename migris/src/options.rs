#[derive(Debug, Default)]
pub struct ReadOptions {
    pub(crate) query: Option<String>,
    pub(crate) table_schema: Option<String>,
    pub(crate) table_name: Option<String>,
}

impl ReadOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_table_schema(mut self, table_schema: impl Into<String>) -> Self {
        self.table_schema = Some(table_schema.into());
        self
    }

    pub fn with_table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }
}

#[derive(Debug, Default)]
pub struct WriteOptions {
    pub(crate) table_schema: Option<String>,
    pub(crate) table_name: Option<String>,
}

impl WriteOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_table_schema(mut self, table_schema: impl Into<String>) -> Self {
        self.table_schema = Some(table_schema.into());
        self
    }

    pub fn with_table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }
}
