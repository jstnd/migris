use std::sync::Arc;

use migris::Driver;

use crate::connections::Connection;

pub struct OpenConnection {
    /// The information for the connection.
    pub connection: Connection,

    /// The driver for the connection.
    pub driver: Arc<dyn Driver>,
}

pub struct QueryProgress {
    complete: usize,
    total: usize,
    value: f32,
}

impl QueryProgress {
    /// Creates a new [`QueryProgress`].
    pub fn new(total: usize) -> Self {
        Self {
            complete: 0,
            total,
            value: 0.0,
        }
    }

    /// Returns the label describing the current progress.
    pub fn label(&self) -> String {
        format!("Running query #{} of {}", self.complete + 1, self.total)
    }

    /// Updates the progress with the given complete number.
    pub fn update(&mut self, complete: usize) {
        self.complete = complete;
        self.value = (complete as f32 / self.total as f32) * 100.0;
    }

    /// Returns the current progress percentage.
    pub fn value(&self) -> f32 {
        self.value
    }
}
