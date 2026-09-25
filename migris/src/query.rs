use std::sync::Arc;

use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

pub struct Query {
    /// The SQL for the query.
    sql: Arc<str>,

    /// The cancellation token for the query.
    token: CancellationToken,
}

impl Query {
    /// Creates a new [`Query`].
    pub fn new(sql: impl Into<String>, token: CancellationToken) -> Self {
        Self {
            sql: Arc::from(sql.into()),
            token,
        }
    }

    /// Returns a [`Future`] that gets fulfilled when query cancellation is requested.
    pub fn cancelled(&self) -> WaitForCancellationFuture<'_> {
        self.token.cancelled()
    }

    /// Returns the SQL for the query.
    pub fn sql(&self) -> Arc<str> {
        self.sql.clone()
    }
}
